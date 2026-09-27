//! Union Merge 纯函数单元测试 · TC-U10~U17（add-git-remote-sync spec：
//! Union Merge 语义；本地序保留 + 远端独有追加 + event_id 去重 + 同 id 冲突拒绝）。

use microstep_lib::sync::{union_merge, SyncError};

/// 最小合法事件行（event_id + 可重放字段）。
fn ev(id: &str) -> String {
    format!("{{\"event_id\":\"{id}\",\"type\":\"TASK_CREATED\",\"id\":\"t-{id}\",\"title\":\"{id}\"}}\n")
}

/// TC-U10 · 远端独有事件并入：本地全部行按原序保留，远端独有行按远端序追加。
#[test]
fn tc_u10_remote_only_events_appended() {
    let local = format!("{}{}", ev("l1"), ev("l2"));
    let remote = format!("{}{}{}", ev("r1"), ev("r2"), ev("r3"));
    let out = union_merge(&local, &remote).unwrap();

    assert_eq!(out.content, format!("{local}{remote}"));
    assert_eq!(out.pulled, 3);
    assert_eq!(out.pushed, 2);
    assert_eq!(out.merged, 0);
}

/// TC-U11 · 同 id 语义相等去重：键序 / 空白不同但解析后 JSON 相等 → 保留本地行。
#[test]
fn tc_u11_same_id_semantic_equal_dedup() {
    let local = "{\"event_id\":\"d1\",\"type\":\"A\",\"x\":1,\"y\":2}\n".to_string();
    // 同 id、同值，但键序打乱 + 冒号后空白差异。
    let remote = "{ \"y\": 2, \"x\": 1, \"type\": \"A\", \"event_id\": \"d1\" }\n".to_string();
    let out = union_merge(&local, &remote).unwrap();

    assert_eq!(out.content, local, "语义相等时保留本地字节形态");
    assert_eq!(out.merged, 1);
    assert_eq!(out.pulled, 0);
    assert_eq!(out.pushed, 0);
}

/// TC-U12 · 同 id 内容冲突拒绝：解析后 JSON 不同 → Err 点名冲突 event_id。
#[test]
fn tc_u12_same_id_conflict_rejected() {
    let local = "{\"event_id\":\"d1\",\"type\":\"A\",\"x\":1}\n".to_string();
    let remote = "{\"event_id\":\"d1\",\"type\":\"A\",\"x\":2}\n".to_string();
    let err = union_merge(&local, &remote).unwrap_err();

    assert_eq!(err, SyncError::Conflict { event_id: "d1".into() });
}

/// TC-U13 · 合并确定性：相同输入重复执行 → 输出字节级一致（含统计）。
#[test]
fn tc_u13_merge_deterministic() {
    let local = format!("{}{}{}", ev("a"), "corrupt-line\n", ev("b"));
    let remote = format!("{}{}", ev("b"), ev("c"));
    let first = union_merge(&local, &remote).unwrap();
    let second = union_merge(&local, &remote).unwrap();

    assert_eq!(first.content, second.content);
    assert_eq!(first.pulled, second.pulled);
    assert_eq!(first.pushed, second.pushed);
    assert_eq!(first.merged, second.merged);
}

/// TC-U14 · 本地无 id 行（损坏行）原样保留（对齐 store.rs 损坏行容错口径）。
#[test]
fn tc_u14_local_unparseable_lines_preserved() {
    let local = format!("{}{}{}", ev("l1"), "not-json {{{\n", ev("l2"));
    let remote = ev("r1");
    let out = union_merge(&local, &remote).unwrap();

    assert_eq!(out.content, format!("{local}{remote}"));
    assert!(out.content.contains("not-json {{{"));
}

/// TC-U15 · 远端无 id 行：仅当字节级不存在于本地时追加；已存在则跳过。
#[test]
fn tc_u15_remote_unparseable_append_rule() {
    let local = "keep-local-junk\n".to_string();
    let remote = "keep-local-junk\nbrand-new-junk\n".to_string();
    let out = union_merge(&local, &remote).unwrap();

    assert_eq!(out.content, "keep-local-junk\nbrand-new-junk\n");
    assert_eq!(out.pulled, 1, "仅字节级新行计入 pulled");
}

/// TC-U16 · 远端 ⊆ 本地：输出 = 本地原文字节不变。
#[test]
fn tc_u16_remote_subset_no_change() {
    let local = format!("{}{}", ev("a"), ev("b"));
    let remote = ev("a");
    let out = union_merge(&local, &remote).unwrap();

    assert_eq!(out.content, local);
    assert_eq!(out.pulled, 0);
    assert_eq!(out.pushed, 1);
    assert_eq!(out.merged, 1);
}

/// TC-U17 · 空输入组合：双空 → 空；本地空 → 远端序全量；远端空 → 本地原文。
#[test]
fn tc_u17_empty_input_combinations() {
    let empty = "";
    let two = format!("{}{}", ev("r1"), ev("r2"));
    let one = ev("l1");

    let both_empty = union_merge(empty, empty).unwrap();
    assert_eq!(both_empty.content, "");
    assert_eq!(
        (both_empty.pulled, both_empty.pushed, both_empty.merged),
        (0, 0, 0)
    );

    let recovery = union_merge(empty, &two).unwrap();
    assert_eq!(recovery.content, two);
    assert_eq!(recovery.pulled, 2);

    let local_only = union_merge(&one, empty).unwrap();
    assert_eq!(local_only.content, one);
    assert_eq!(local_only.pushed, 1);
}
