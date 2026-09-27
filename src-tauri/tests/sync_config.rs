//! 同步配置（sync.json）单元测试 · TC-U01~U05（add-git-remote-sync spec：
//! 同步配置持久化）。

use std::fs;
use std::time::{SystemTime, UNIX_EPOCH};

use microstep_lib::sync::{
    apply_sync_config_update, load_sync_config, mask_pat, save_sync_config, SyncConfig,
    SyncConfigUpdate,
};

fn unique_dir(tag: &str) -> std::path::PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    std::env::temp_dir().join(format!("microstep-synccfg-{tag}-{nanos}"))
}

/// TC-U01 · 保存并脱敏回显：save → load 字段逐项还原；PAT 掩码仅保留末 4 位。
#[test]
fn tc_u01_save_and_masked_echo() {
    let dir = unique_dir("u01");
    let cfg = SyncConfig {
        remote_url: "https://github.com/u/microstep-data.git".into(),
        pat: "ghp_abcd1234".into(),
        branch: "main".into(),
        last_sync_at: None,
        last_result: None,
    };
    save_sync_config(&dir, &cfg).unwrap();

    // 配置落盘 sync.json（与 events.jsonl 同目录），键名与规格一致。
    let raw = fs::read_to_string(dir.join("sync.json")).unwrap();
    let raw: serde_json::Value = serde_json::from_str(&raw).unwrap();
    assert_eq!(raw["remote_url"], "https://github.com/u/microstep-data.git");
    assert_eq!(raw["pat"], "ghp_abcd1234");
    assert_eq!(raw["branch"], "main");

    let loaded = load_sync_config(&dir);
    assert_eq!(loaded.remote_url, cfg.remote_url);
    assert_eq!(loaded.pat, cfg.pat);
    assert_eq!(loaded.branch, cfg.branch);

    // 脱敏：仅末 4 位可辨识，前缀不可泄露。
    let masked = mask_pat("ghp_abcd1234");
    assert!(masked.ends_with("1234"), "掩码应保留末 4 位: {masked}");
    assert!(!masked.contains("ghp_abcd"), "掩码不得泄露前缀: {masked}");
}

/// TC-U02 · 未配置 = 同步未启用：sync.json 不存在 → 默认空配置（branch=main），
/// 读取不报错（引擎据此判定未启用）。
#[test]
fn tc_u02_missing_config_means_disabled() {
    let dir = unique_dir("u02");
    let cfg = load_sync_config(&dir);
    assert_eq!(cfg.remote_url, "");
    assert_eq!(cfg.pat, "");
    assert_eq!(cfg.branch, "main");
    assert_eq!(cfg.last_sync_at, None);
    assert_eq!(cfg.last_result, None);
}

/// TC-U03 · 部分更新语义：未携带字段保持原值；pat 显式空串清除既有 PAT。
#[test]
fn tc_u03_partial_update_semantics() {
    let dir = unique_dir("u03");
    save_sync_config(
        &dir,
        &SyncConfig {
            remote_url: "https://example.com/a.git".into(),
            pat: "token-xyz".into(),
            branch: "main".into(),
            last_sync_at: None,
            last_result: None,
        },
    )
    .unwrap();

    // 仅更新 branch：url / pat 不变。
    let mut cfg = load_sync_config(&dir);
    apply_sync_config_update(
        &mut cfg,
        &SyncConfigUpdate {
            remote_url: None,
            pat: None,
            branch: Some("dev".into()),
        },
    );
    save_sync_config(&dir, &cfg).unwrap();
    let cfg = load_sync_config(&dir);
    assert_eq!(cfg.remote_url, "https://example.com/a.git");
    assert_eq!(cfg.pat, "token-xyz");
    assert_eq!(cfg.branch, "dev");

    // pat 显式传空字符串 → 清除既有 PAT。
    let mut cfg = load_sync_config(&dir);
    apply_sync_config_update(
        &mut cfg,
        &SyncConfigUpdate {
            remote_url: None,
            pat: Some(String::new()),
            branch: None,
        },
    );
    save_sync_config(&dir, &cfg).unwrap();
    assert_eq!(load_sync_config(&dir).pat, "");

    // 未携带 pat（None）→ 不清除。
    let mut cfg = load_sync_config(&dir);
    apply_sync_config_update(
        &mut cfg,
        &SyncConfigUpdate {
            remote_url: Some("https://example.com/b.git".into()),
            pat: None,
            branch: None,
        },
    );
    assert_eq!(cfg.pat, "");
    assert_eq!(cfg.remote_url, "https://example.com/b.git");
}

/// TC-U04 · 默认值：首次保存（仅携带 remote_url）→ branch 默认 main，
/// last_sync_at / last_result 初始为空。
#[test]
fn tc_u04_default_branch_and_empty_history() {
    let dir = unique_dir("u04");
    let mut cfg = load_sync_config(&dir); // 默认配置
    apply_sync_config_update(
        &mut cfg,
        &SyncConfigUpdate {
            remote_url: Some("https://example.com/c.git".into()),
            pat: Some("pat1234".into()),
            branch: None,
        },
    );
    save_sync_config(&dir, &cfg).unwrap();

    let cfg = load_sync_config(&dir);
    assert_eq!(cfg.branch, "main");
    assert_eq!(cfg.last_sync_at, None);
    assert_eq!(cfg.last_result, None);
}

/// TC-U05 · PAT 脱敏边界（假设 A1）：长度 ≤4 的 PAT 全掩码，不完整泄露；
/// 空串回显为空。
#[test]
fn tc_u05_short_pat_fully_masked() {
    assert_eq!(mask_pat(""), "");
    for short in ["a", "ab", "abc", "abcd"] {
        let masked = mask_pat(short);
        assert!(!masked.contains(short), "短 PAT 必须全掩码: {short} -> {masked}");
    }
    // 5 位起仅保留末 4 位。
    let masked = mask_pat("x1234");
    assert!(masked.ends_with("1234"));
    assert!(!masked.contains('x'));
}
