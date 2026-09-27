//! data_dir 模块测试（TC-I26~I28）：目录与 .git 骨架初始化、既有仓库幂等、
//! 存量事件文件不覆盖（data-storage spec「不覆盖保护」场景，端到端含 EventStore）。

use std::fs;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::json;

use microstep_lib::app_state::AppState;
use microstep_lib::data_dir;
use microstep_lib::store::{Clock, ClockNow, EventStore};

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
    // reflog 目录树：config 开启 logallrefupdates，fetch/push 写 reflog
    // 需父目录就位（Linux 上 gix/git2 不自动逐级创建，回归钉死）。
    assert!(dir.join(".git/refs/remotes").is_dir());
    assert!(dir.join(".git/logs/refs/heads").is_dir());
    assert!(dir.join(".git/logs/refs/remotes").is_dir());
    // 骨架内置同步身份：gix ref 事务写 reflog 需 committer 签名，无全局
    // git 身份的环境（CI 全新 runner / 未装 git 的终端用户）必须自给自足。
    let config = fs::read_to_string(dir.join(".git/config")).unwrap();
    assert!(config.contains("[user]"), "骨架 config 须含 [user] 段");
    assert!(config.contains("name = MicroStep"));
    assert!(config.contains("email = sync@microstep.local"));
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

/// TC-I29 · 存量骨架自愈：config 缺 [user] 段时补同步身份（gix ref 事务写
/// reflog 需 committer；无全局 git 身份环境 ref 更新会整体失败，ERR-002 根因），
/// 既有段落与 HEAD 原样保留（仅追加，不覆盖）。
#[test]
fn tc_i29_self_heal_missing_user_identity() {
    let dir = unique_dir("i29");
    fs::create_dir_all(dir.join(".git/refs/heads")).unwrap();
    fs::write(dir.join(".git/HEAD"), "ref: refs/heads/main\n").unwrap();
    fs::write(
        dir.join(".git/config"),
        "[core]\n\trepositoryformatversion = 0\n\tfilemode = false\n\tbare = false\n\tlogallrefupdates = true\n",
    )
    .unwrap();
    data_dir::init_data_dir(&dir);
    assert_eq!(
        fs::read_to_string(dir.join(".git/HEAD")).unwrap(),
        "ref: refs/heads/main\n",
        "HEAD 不被改写"
    );
    let config = fs::read_to_string(dir.join(".git/config")).unwrap();
    assert!(config.contains("[core]"), "原有配置段落保留");
    assert!(config.contains("logallrefupdates = true"), "原有配置值保留");
    assert!(config.contains("[user]"), "自愈补 [user] 段");
    assert!(config.contains("name = MicroStep"));
    assert!(config.contains("email = sync@microstep.local"));
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
