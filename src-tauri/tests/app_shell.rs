//! app-shell 移动就绪守卫测试（TC-S01/S02）：single-instance 为桌面专属
//! 插件，注册必须由 `#[cfg(desktop)]` 语句级门控；移动入口属性不得丢失
//! （add-cicd-multiplatform app-shell 增量）。以纯 std 源码静态审计实现，
//! 不引入依赖；移动目标真实编译回归由 ci.yml 的
//! `cargo check --target aarch64-linux-android --locked` 承担。

use std::fs;

fn lib_rs() -> String {
    fs::read_to_string("src/lib.rs").expect("读取 src/lib.rs 失败")
}

/// TC-S01 · single-instance 注册缺守卫即失败：定位 `init` 调用所属语句，
/// 向上（跳过空行/纯注释行）扫描语句级属性，要求存在 `#[cfg(desktop)]`。
#[test]
fn tc_s01_single_instance_registration_is_desktop_gated() {
    let src = lib_rs();
    let lines: Vec<&str> = src.lines().collect();

    let init_line = lines
        .iter()
        .position(|l| l.contains("tauri_plugin_single_instance::init"))
        .expect("lib.rs 必须注册 single-instance 插件（桌面目标）");

    let gated = lines[..init_line]
        .iter()
        .rev()
        .filter(|l| !l.trim().is_empty())
        .filter(|l| !l.trim_start().starts_with("//"))
        .take(3)
        .any(|l| l.trim() == "#[cfg(desktop)]");
    assert!(
        gated,
        "single-instance 插件注册必须由 #[cfg(desktop)] 语句级守卫包裹 \
         （app-shell「桌面插件编译门控」：移动目标不可编译进桌面专属插件）"
    );
}

/// TC-S02 · 移动入口保留：iOS 走 `mobile_entry_point`；Android 因需插入
/// rustls-platform-verifier 初始化（ADR-008）改为手写展开——run() 标注
/// iOS 入口属性，且存在 `android_binding!` 的 `_start_app` 展开与
/// `stop_unwind` 防护（双平台入口缺失即红）。
#[test]
fn tc_s02_mobile_entry_point_attr_preserved() {
    let source = lib_rs();
    assert!(
        source.contains("#[cfg_attr(target_os = \"ios\", tauri::mobile_entry_point)]"),
        "run() 必须保留 iOS 的 mobile_entry_point 入口属性"
    );
    assert!(
        source.contains("tauri::android_binding!(com_microstep, app, _start_app,"),
        "Android 必须保留 android_binding! 入口（ADR-008 TLS 初始化 shim）"
    );
    assert!(
        source.contains("fn stop_unwind<F: FnOnce() -> T, T>(f: F) -> T"),
        "Android 手写入口必须保留 panic 防护（与 mobile_entry_point 展开一致）"
    );
}
