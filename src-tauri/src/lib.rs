//! MicroStep 2.0 · Tauri v2 壳 + Rust 领域核心。
//!
//! 领域层（`domain.rs` / `store.rs`）移植自 Python `backend/domain.py` /
//! `backend/store.py`，行为等价性由 `tests/golden.rs` 的 golden replay 对照守卫。
//! 壳层（`app_state.rs` / `commands.rs` / `data_dir.rs` / `ticker.rs`）镜像
//! `backend/server.py`：AppData 落盘 + 手工 git 骨架、17 个 IPC command
//! 信封兼容、single-instance 防并发写、tokio Ticker 后台补每日结算。

pub mod app_state;
pub mod commands;
pub mod data_dir;
pub mod domain;
pub mod store;
pub mod ticker;
pub mod sync;

use std::sync::Arc;
use std::time::Duration;

use tauri::Manager;

use crate::app_state::AppState;
use crate::store::EventStore;

/// Ticker 检查周期（对齐 Python `Ticker` 默认 600s）。
const TICK_INTERVAL: Duration = Duration::from_secs(600);

/// 桌面 / iOS 入口：single-instance（须最先注册）→ setup（AppData 初始化 +
/// 启动补结算 + 启动 best-effort pull + Ticker）→ 17 个 command（ipc-api spec
/// 白名单）。Android 不走 mobile_entry_point（其展开会固定把 wry 的
/// android_setup 作 setup 钩子），改用下方 `_start_app` 手写展开以插入
/// rustls-platform-verifier 初始化（ADR-008）。
#[cfg_attr(target_os = "ios", tauri::mobile_entry_point)]
pub fn run() {
    let builder = tauri::Builder::default();
    // single-instance 为桌面专属插件（插件 crate 在 android/ios 目标整体
    // cfg 掉）：语句级 #[cfg(desktop)] 门控保证移动目标可编译，桌面注册
    // 顺序不变（须最先注册）。
    #[cfg(desktop)]
    let builder = builder.plugin(tauri_plugin_single_instance::init(|app, _argv, _cwd| {
        // 二次启动：焦点转交既有主窗口后新进程自行退出（插件行为）。
        if let Some(window) = app.get_webview_window("main") {
            let _ = window.unminimize();
            let _ = window.set_focus();
        }
    }));
    builder
        .setup(|app| {
            let data_dir = app
                .path()
                .app_data_dir()
                .expect("定位 AppData 数据目录失败");
            let events = data_dir::init_data_dir(&data_dir);
            let state = Arc::new(AppState::new(EventStore::open(events)));
            state.ensure_bootstrapped(); // 启动补结算（background-ticker spec）
            // 启动 best-effort pull：异步执行，失败仅记 last_result（不阻塞不弹窗）。
            let pull_state = Arc::clone(&state);
            std::thread::spawn(move || pull_state.startup_pull());
            app.manage(state.clone());
            tauri::async_runtime::spawn(ticker::run(state, TICK_INTERVAL));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_state,
            commands::get_meta,
            commands::create_epic,
            commands::update_epic,
            commands::complete_epic,
            commands::create_task,
            commands::log_task,
            commands::update_task,
            commands::delete_task,
            commands::equip_title,
            commands::unequip_title,
            commands::awaken,
            commands::system_tick,
            commands::delete_event,
            commands::sync_get_config,
            commands::sync_set_config,
            commands::sync_now
        ])
        .run(tauri::generate_context!())
        .expect("Tauri 应用启动失败");
}

// ---------------------------------------------------------------------------
// Android：TLS 信任根初始化（ADR-008）
// ---------------------------------------------------------------------------

/// Android 入口：mobile_entry_point 的手写展开（tauri-macros 2.6.3 mobile.rs），
/// 唯一差异 = `$wry` 模块换成下方 `android_wry_glue` shim。`com_microstep` /
/// `app` 由 tauri.conf.json identifier `com.microstep.app` 推导（与编译期
/// env TAURI_ANDROID_PACKAGE_NAME_PREFIX / APP_NAME 的字面值一致），改 identifier
/// 时需同步。
#[cfg(target_os = "android")]
fn _start_app() {
    ::tauri::android_binding!(com_microstep, app, _start_app, android_wry_glue);
    stop_unwind(run);
}

/// 与 mobile_entry_point 展开一致的 panic 防护（Rust 侧 unwind 不得穿出 JNI）。
#[cfg(target_os = "android")]
fn stop_unwind<F: FnOnce() -> T, T>(f: F) -> T {
    match std::panic::catch_unwind(std::panic::AssertUnwindSafe(f)) {
        Ok(t) => t,
        Err(err) => {
            eprintln!("attempt to unwind out of `rust` with err: {:?}", err);
            std::process::abort()
        }
    }
}

/// `tauri::android_binding!` 的 `$wry` shim：原样转发 wry 的 prelude 与
/// android_setup，但在 setup 前用同一 JNIEnv/Activity 初始化
/// rustls-platform-verifier——reqwest 0.13 rustls 特性在 Android 的唯一证书
/// 验证器。未初始化时首次 HTTPS 证书校验直接 panic（rc2 MuMu 同步排障根因，
/// 表现为被遮蔽的 "An IO error occurred..."）。
#[cfg(target_os = "android")]
pub(crate) mod android_wry_glue {
    pub use tauri::wry::prelude;

    use tauri::wry::prelude::{ndk, GlobalRef, JNIEnv};

    /// 与 `wry::android_setup` 同签名（tao android_binding! 的 setup 钩子）。
    pub unsafe fn android_setup(
        package: &str,
        mut env: JNIEnv,
        looper: &ndk::looper::ThreadLooper,
        activity: GlobalRef,
    ) {
        match init_platform_verifier(&mut env, &activity) {
            Ok(()) => eprintln!("AGENT-DEBUG: rustls-platform-verifier initialized"),
            Err(detail) => eprintln!("AGENT-DEBUG: rustls-platform-verifier init failed: {detail}"),
        }
        tauri::wry::android_setup(package, env, looper, activity)
    }

    /// jni 0.21（tao/wry 栈）→ 0.22（rustls-platform-verifier 栈）桥接：
    /// 两个大版本包装同一套 JNI C ABI，按裸指针互通（版本间无所有权转移，
    /// EnvUnowned 不 detach 线程、不销毁 VM）。
    fn init_platform_verifier(env: &mut JNIEnv, activity: &GlobalRef) -> Result<(), String> {
        let raw_env = env.get_raw() as *mut jni::sys::JNIEnv;
        let raw_activity = activity.as_obj().as_raw() as jni::sys::jobject;
        // SAFETY: env 是当前线程的有效 JNI 附着（setup 回调持有）；
        // EnvUnowned::from_raw 仅包装既有指针，不获取所有权。
        let mut unowned = unsafe { jni::EnvUnowned::from_raw(raw_env) };
        let outcome = unowned.with_env(|env| {
            // SAFETY: raw_activity 指向 activity 全局引用的对象，本调用内存活。
            let context = unsafe { jni::objects::JObject::from_raw(env, raw_activity) };
            rustls_platform_verifier::android::init_with_env(env, context)
                .map_err(|err| Box::new(err) as Box<dyn std::error::Error>)
        });
        match outcome.into_outcome() {
            jni::Outcome::Ok(()) => Ok(()),
            jni::Outcome::Err(err) => Err(err.to_string()),
            jni::Outcome::Panic(payload) => Err(format!("init panicked: {payload:?}")),
        }
    }
}
