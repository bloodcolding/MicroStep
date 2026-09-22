//! data_dir 模块测试（TC-I26~I28）：目录与 .git 骨架初始化、既有仓库幂等、
//! 存量事件文件不覆盖（data-storage spec「不覆盖保护」场景，端到端含 EventStore）。

use std::fs;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::json;

use microstep::app_state::AppState;
use microstep::data_dir;
use microstep::store::{Clock, ClockNow, EventStore};

fn fixed_clock(date: &str) -> Clock {
    let date = date.to_string();
    Arc::new(move || ClockNow {
        date: date.clone(),
        datetime: format!("{date}T12:00:00+08:00"),
    })
}

fn unique_dir(tag: &str) -> PathBuf {
    let nanos = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
    std::env::temp_dir().join(format!("microstep-datadir-{tag}-{nanos}"))
}

/// TC-I26 · 首次初始化：创建数据目录与 .git 骨架（HEAD + objects + refs 齐备）。
#[test]
fn tc_i26_init_creates_dir_and_git_skeleton() {
    let dir = unique_dir("i26");
    let events = data_dir::init_data_dir(&dir);
    assert!(dir.is_dir());
    assert!(dir.join(".git/HEAD").exists());
    assert!(dir.join(".git/config").exists());
    assert!(dir.join(".git/objects/info").is_dir());
    assert!(dir.join(".git/objects/pack").is_dir());
    assert!(dir.join(".git/refs/heads").is_dir());
    assert!(dir.join(".git/refs/tags").is_dir());
    assert_eq!(events, dir.join("events.jsonl"));
}

/// TC-I27 · 已是 git 仓库的目录不覆盖：预置 marker HEAD 保持原样（幂等跳过）。
#[test]
fn tc_i27_existing_git_repo_untouched() {
    let dir = unique_dir("i27");
    fs::create_dir_all(dir.join(".git")).unwrap();
    fs::write(dir.join(".git/HEAD"), "ref: refs/heads/master\n").unwrap();
    data_dir::init_data_dir(&dir);
    assert_eq!(fs::read_to_string(dir.join(".git/HEAD")).unwrap(), "ref: refs/heads/master\n");
}

/// TC-I28 · 存量数据不覆盖：已有 events.jsonl 的目录初始化后重放续写，
/// 不重置、不重复注入 SYSTEM_INIT（迁移拷贝场景）。
#[test]
fn tc_i28_existing_events_not_reset() {
    let dir = unique_dir("i28");
    fs::create_dir_all(&dir).unwrap();
    fs::write(
        dir.join("events.jsonl"),
        concat!(
            r#"{"type":"SYSTEM_INIT","name":"系统初始化","changes":[],"date":"2026-01-01","at":"2026-01-01T08:00:00+08:00","event_id":"evt-1","created_at":"2026-01-01T08:00:00+08:00"}"#,
            "\n",
            r#"{"type":"SYSTEM_DAILY_TICK","name":"每日结算","changes":[],"date":"2026-01-01","at":"2026-01-01T08:00:01+08:00","event_id":"evt-2","created_at":"2026-01-01T08:00:01+08:00"}"#,
            "\n",
        ),
    )
    .unwrap();

    let events = data_dir::init_data_dir(&dir);
    let app = AppState::new(EventStore::with_clock(events, fixed_clock("2026-01-02")));
    let payload = app.get_state();
    // 重放存量 + 补当日 Tick（最后 tick 为 01-01 → 追加 01-02 一条）。
    assert_eq!(payload["meta"]["last_tick_day"], json!("2026-01-02"));

    let content = fs::read_to_string(dir.join("events.jsonl")).unwrap();
    assert_eq!(content.matches("SYSTEM_INIT").count(), 1, "不得重复初始化");
    assert!(content.contains("2026-01-02"), "应续写补结算 Tick");
    assert_eq!(content.lines().count(), 3, "2 条存量 + 1 条补 Tick");
}
