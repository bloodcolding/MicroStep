//! 同步 IPC 契约与错误分类测试 · TC-U20~U24 / TC-I20~I22（add-git-remote-sync
//! spec：错误处理与离线可用 + ipc-api 增量）。

use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::{json, Value};

use microstep::app_state::AppState;
use microstep::data_dir;
use microstep::store::{Clock, ClockNow, EventStore};
use microstep::sync::{save_sync_config, SyncConfig, SyncError};

fn fixed_clock(date: &str) -> Clock {
    let date = date.to_string();
    Arc::new(move || ClockNow {
        date: date.clone(),
        datetime: format!("{date}T12:00:00+08:00"),
    })
}

fn unique_dir(tag: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    std::env::temp_dir().join(format!("microstep-syncipc-{tag}-{nanos}"))
}

/// 带数据目录（.git 骨架）的 AppState，供同步信封方法使用。
fn sync_app_state(tag: &str) -> (AppState, PathBuf) {
    let dir = unique_dir(tag);
    let events = data_dir::init_data_dir(&dir);
    (
        AppState::new(EventStore::with_clock(events, fixed_clock("2026-01-05"))),
        dir,
    )
}

fn task_body() -> Value {
    json!({
        "title": "测试 Task",
        "epic_id": "",
        "repeatable": false,
        "effects": [{"dimension": "knowledge", "delta": 5}],
    })
}

fn init_bare(tag: &str) -> PathBuf {
    let path = unique_dir(tag);
    git2::Repository::init_bare(&path).expect("init bare repo");
    path
}

// ---------------------------------------------------------------------------
// 错误分类（TC-U20~U24）：SyncError → 信封 error 文案
// ---------------------------------------------------------------------------

/// TC-U20 · 网络失败 / 超时 → 网络类文案（spec：网络失败（含超时）为一类）。
#[test]
fn tc_u20_network_and_timeout_messages() {
    assert!(SyncError::Network("connection refused".into())
        .message()
        .contains("网络"));
    assert!(SyncError::Timeout("300ms".into())
        .message()
        .contains("超时"));
}

/// TC-U21 · 认证失败映射：401/403 → Auth 分类且提示检查 PAT；其他 HTTP 状态归网络。
#[test]
fn tc_u21_auth_mapping_401_403() {
    for status in [401u16, 403] {
        let err = SyncError::from_http_status(status);
        assert!(matches!(err, SyncError::Auth(_)), "{status} 应映射为认证失败");
        assert!(
            err.message().contains("PAT"),
            "认证失败文案应提示检查 PAT: {}",
            err.message()
        );
    }
    assert!(matches!(
        SyncError::from_http_status(500),
        SyncError::Network(_)
    ));
}

/// TC-U22 · 未配置错误文案。
#[test]
fn tc_u22_not_configured_message() {
    assert!(SyncError::NotConfigured.message().contains("未配置"));
}

/// TC-U23 · 同 id 冲突文案点名 event_id。
#[test]
fn tc_u23_conflict_message_names_event_id() {
    let msg = SyncError::Conflict {
        event_id: "e42".into(),
    }
    .message();
    assert!(msg.contains("e42"), "冲突错误必须点名 event_id: {msg}");
}

/// TC-U24 · 远端并发更新文案。
#[test]
fn tc_u24_concurrent_update_message() {
    assert!(
        SyncError::ConcurrentUpdate("non-fast-forward".into())
            .message()
            .contains("并发")
    );
}

// ---------------------------------------------------------------------------
// IPC 信封（TC-I20 / TC-I22）
// ---------------------------------------------------------------------------

/// TC-I20 · sync_get_config / sync_set_config 信封：保存 → 脱敏回显 → 部分更新。
#[test]
fn tc_i20_sync_config_envelopes() {
    let (app, dir) = sync_app_state("i20");

    // 未配置：ok:true + 空配置（branch 默认 main）。
    let payload = app.sync_get_config();
    assert_eq!(payload["ok"], json!(true));
    assert_eq!(payload["remote_url"], json!(""));
    assert_eq!(payload["branch"], json!("main"));
    assert_eq!(payload["last_sync_at"], json!(null));

    // 保存（不携带 branch → 保持默认）。
    let saved = app.sync_set_config(
        Some("https://example.com/r.git".into()),
        Some("ghp_abcd1234".into()),
        None,
    );
    assert_eq!(saved["ok"], json!(true));

    let payload = app.sync_get_config();
    assert_eq!(payload["ok"], json!(true));
    assert_eq!(payload["remote_url"], json!("https://example.com/r.git"));
    assert_eq!(payload["branch"], json!("main"), "未携带 branch 保持默认");
    let pat = payload["pat"].as_str().expect("pat 字段");
    assert!(pat.ends_with("1234"), "PAT 仅末 4 位可辨识: {pat}");
    assert!(!pat.contains("ghp_abcd"), "PAT 前缀不得泄露: {pat}");
    assert!(dir.join("sync.json").exists(), "配置应落盘 sync.json");

    // 部分更新：清 PAT + 改分支，URL 不动。
    app.sync_set_config(None, Some(String::new()), Some("dev".into()));
    let payload = app.sync_get_config();
    assert_eq!(payload["pat"], json!(""), "pat 空串清除");
    assert_eq!(payload["branch"], json!("dev"));
    assert_eq!(payload["remote_url"], json!("https://example.com/r.git"));
}

/// TC-I22 · sync_now 信封：未配置 → ok:false 未配置文案；成功 → ok:true 含
/// pulled/pushed/merged 数字统计，attempts 等内部字段不得外泄。
#[test]
fn tc_i22_sync_now_envelope_stats() {
    // 未配置。
    let (app, _dir) = sync_app_state("i22a");
    let payload = app.sync_now();
    assert_eq!(payload["ok"], json!(false));
    assert!(
        payload["error"].as_str().is_some_and(|e| e.contains("未配置")),
        "错误信封应含未配置文案: {payload}"
    );

    // 配置到本地空 bare 远端，先造业务事件再同步。
    let bare = init_bare("i22b");
    let (app, dir) = sync_app_state("i22b");
    let created = app.create_task(&task_body());
    assert_eq!(created["ok"], json!(true));
    save_sync_config(
        &dir,
        &SyncConfig {
            remote_url: bare.to_string_lossy().into_owned(),
            pat: String::new(),
            branch: "main".into(),
            last_sync_at: None,
            last_result: None,
        },
    )
    .unwrap();

    let payload = app.sync_now();
    assert_eq!(payload["ok"], json!(true), "同步失败: {payload}");
    assert!(payload["pulled"].is_u64(), "pulled 统计: {payload}");
    assert!(
        payload["pushed"].as_u64().is_some_and(|n| n >= 1),
        "本地独有事件应计入 pushed: {payload}"
    );
    assert!(payload["merged"].is_u64(), "merged 统计: {payload}");
    assert!(
        payload.get("attempts").is_none(),
        "attempts 为内部字段，不得进 IPC 信封: {payload}"
    );
}

// ---------------------------------------------------------------------------
// 命令集完备审计（TC-I21，ipc-api spec：恰好 17 个 command）
// ---------------------------------------------------------------------------

/// TC-I21 · 命令注册表审计：generate_handler 恰含 17 个 command（14 既有 +
/// 3 同步），无多余业务 command。
#[test]
fn tc_i21_exactly_17_commands_registered() {
    let src = include_str!("../src/lib.rs");
    let start = src.find("generate_handler!").expect("generate_handler 块");
    let block = &src[start..];
    let registered: HashSet<String> = block
        .lines()
        .filter_map(|line| line.trim().strip_prefix("commands::"))
        .map(|name| name.trim_end_matches(',').to_string())
        .collect();

    let expected: HashSet<String> = [
        "get_state",
        "get_meta",
        "create_epic",
        "update_epic",
        "complete_epic",
        "create_task",
        "log_task",
        "update_task",
        "delete_task",
        "equip_title",
        "unequip_title",
        "awaken",
        "system_tick",
        "delete_event",
        "sync_get_config",
        "sync_set_config",
        "sync_now",
    ]
    .iter()
    .map(|s| s.to_string())
    .collect();

    assert_eq!(
        registered, expected,
        "IPC command 注册表必须恰为 17 个（14 既有 + 3 同步）"
    );
}
