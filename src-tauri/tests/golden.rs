//! TC-I23/24/25：golden replay 等价验收（domain-rules spec 核心门禁）。
//!
//! 基准资产由 Python 版导出（`tests/export_golden.py`，tasks 2.1/2.2）：
//! - 真实事件流快照 + 覆盖全部 14 种事件的构造序列（固定日期 2026-09-19/20/21）；
//! - Rust 版重放相同输入，canonical（递归键排序/紧凑分隔符/非 ASCII 不转义）
//!   对比零差异 = 移植行为等价；资产随仓库常驻，作为后续规则修改的守门测试。

use microstep::domain::{build_state, canonical_state_json, Event};

fn replay_asset(asset: &str) -> String {
    let events: Vec<Event> = asset
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| {
            let value = serde_json::from_str(line).expect("资产行必须是合法 JSON");
            Event::from_value(value)
        })
        .collect();
    assert!(!events.is_empty(), "基准输入不得为空");
    canonical_state_json(&build_state(&events))
}

/// TC-I23：真实事件流（domain-rules spec「真实数据等价」场景）。
#[test]
fn golden_real_replay_matches_baseline() {
    let events = include_str!("assets/golden_real_events.jsonl");
    let baseline = include_str!("assets/golden_real_state.json");
    assert_eq!(replay_asset(events), baseline.trim_end());
}

/// TC-I24：14 种事件构造序列（domain-rules spec「全事件类型构造序列等价」场景）。
#[test]
fn golden_synthetic_replay_matches_baseline() {
    let events = include_str!("assets/golden_synthetic_events.jsonl");
    let baseline = include_str!("assets/golden_synthetic_state.json");
    assert_eq!(replay_asset(events), baseline.trim_end());
}

/// TC-I25：基准回归常驻守门——两套资产齐备且构造序列覆盖全部 14 种事件
/// （domain-rules spec「基准回归常驻」场景）。
#[test]
fn golden_assets_guard() {
    let synthetic = include_str!("assets/golden_synthetic_events.jsonl");
    let mut types = std::collections::BTreeSet::new();
    for line in synthetic.lines().filter(|line| !line.trim().is_empty()) {
        let value: serde_json::Value = serde_json::from_str(line).expect("合法 JSON");
        types.insert(value["type"].as_str().expect("事件必须有 type").to_string());
    }
    assert_eq!(types.len(), 14, "构造序列必须恰好覆盖 14 种事件，实际: {types:?}");
    assert!(
        !include_str!("assets/golden_real_state.json").trim().is_empty(),
        "真实流基准快照不得为空"
    );
}
