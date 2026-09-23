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

/// 桌面入口：single-instance（须最先注册）→ setup（AppData 初始化 + 启动补结算
/// + 启动 best-effort pull + Ticker）→ 17 个 command（ipc-api spec 白名单）。
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _argv, _cwd| {
            // 二次启动：焦点转交既有主窗口后新进程自行退出（插件行为）。
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.unminimize();
                let _ = window.set_focus();
            }
        }))
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
