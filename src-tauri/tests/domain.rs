//! TC-U01~U26：`test_domain.py` 26 例逐条移植（用例名/断言语义不变）。
//! TC-U45~U49：Rust 侧新增护栏（serde 往返、Unknown 兜底、时钟语义钉死）。
//! 事件构造沿用 Python 测试的 dict 字面量风格（经 `Event::from_value` 解析），
//! 同时持续锤炼 golden replay 依赖的 serde tag 映射。

use std::thread::sleep;
use std::time::Duration;

use serde_json::{json, Value};

use microstep_lib::domain::{
    apply_event, apply_title_bonuses, build_state, canonical_state_json, initial_state, pool_dimensions,
    has_dimension, Event, State,
};

fn ev(value: Value) -> Event {
    Event::from_value(value)
}

fn make_task_created(task_id: &str) -> Value {
    json!({
        "type": "TASK_CREATED",
        "id": task_id,
        "name": format!("创建 Task：{task_id}"),
        "changes": [],
        "title": task_id,
        "epic_id": null,
        "effects": [{"dimension": "knowledge", "delta": 5.0}],
        "repeatable": false,
        "date": "2026-09-19",
        "at": "2026-09-19T10:00:00",
    })
}

fn make_task_completed(task_id: &str, effects: Value) -> Value {
    json!({
        "type": "TASK_COMPLETED",
        "id": task_id,
        "name": format!("记录 Task：{task_id}"),
        "changes": effects,
        "effects": effects,
        "note": "",
        "date": "2026-09-19",
        "at": "2026-09-19T10:05:00",
    })
}

fn make_epic_created(epic_id: &str) -> Value {
    json!({
        "type": "EPIC_CREATED",
        "id": epic_id,
        "name": format!("创建里程碑：{epic_id}"),
        "changes": [],
        "title": epic_id,
        "description": "",
        "main_dimension": "professional",
        "title_emoji": "🏅",
        "title_bonus_dimension": "professional",
        "title_bonus_percent": 10,
        "date": "2026-09-19",
        "at": "2026-09-19T10:00:00",
    })
}

fn fx(dimension: &str, delta: f64) -> microstep_lib::domain::EffectInput {
    microstep_lib::domain::EffectInput { dimension: dimension.to_string(), delta }
}

fn dim(state: &State, key: &str) -> f64 {
    state
        .dimensions
        .get(key)
        .unwrap_or_else(|| panic!("维度缺失: {key}"))
        .as_f64()
        .expect("维度值必须是数字")
}

// ---------------------------------------------------------------------------
// InitialStateTests
// ---------------------------------------------------------------------------

/// TC-U01 · ported from tests/test_domain.py::test_dimensions_shape
#[test]
fn test_dimensions_shape() {
    let state = initial_state();
    let mut keys = state.dimensions.keys().map(String::as_str).collect::<Vec<_>>();
    keys.sort_unstable();
    let mut expected = vec!["san"];
    expected.extend(pool_dimensions());
    expected.sort_unstable();
    assert_eq!(keys, expected);
    assert_eq!(dim(&state, "san"), 100.0);
    for key in pool_dimensions() {
        assert_eq!(dim(&state, key), 0.0);
    }
}

/// TC-U02 · ported from tests/test_domain.py::test_no_willpower_or_level_system
#[test]
fn test_no_willpower_or_level_system() {
    let state = initial_state();
    assert!(!state.dimensions.contains_key("willpower"));
    assert!(!state.derived.pool_dimensions.contains(&"willpower".to_string()));
    assert!(!has_dimension("willpower"));
}

// ---------------------------------------------------------------------------
// TaskReducerTests
// ---------------------------------------------------------------------------

/// TC-U03 · ported from tests/test_domain.py::test_task_created_registers_task_and_epic_link
#[test]
fn test_task_created_registers_task_and_epic_link() {
    let mut state = initial_state();
    apply_event(&mut state, &ev(make_epic_created("e1")));
    let mut created = make_task_created("t1");
    created["epic_id"] = json!("e1");
    apply_event(&mut state, &ev(created));
    let task = state.tasks.get("t1").unwrap();
    assert_eq!(task.status, "active");
    assert_eq!(task.effects.len(), 1);
    assert_eq!(task.effects[0].dimension, "knowledge");
    assert_eq!(task.effects[0].delta, 5.0);
    assert!(state.epics.get("e1").unwrap().task_ids.contains(&"t1".to_string()));
}

/// TC-U04 · ported from tests/test_domain.py::test_task_completed_settles_effects
#[test]
fn test_task_completed_settles_effects() {
    let mut state = initial_state();
    apply_event(&mut state, &ev(make_task_created("t1")));
    apply_event(&mut state, &ev(make_task_completed("t1", json!([{"dimension": "knowledge", "delta": 5.0}]))));
    assert_eq!(dim(&state, "knowledge"), 5.0);
    assert_eq!(state.tasks.get("t1").unwrap().status, "completed");
    assert_eq!(state.logs.len(), 1);
}

/// TC-U05 · ported from tests/test_domain.py::test_pool_clamps_at_zero
#[test]
fn test_pool_clamps_at_zero() {
    let mut state = initial_state();
    apply_event(&mut state, &ev(make_task_created("t1")));
    apply_event(
        &mut state,
        &ev(make_task_completed("t1", json!([{"dimension": "physical", "delta": -50.0}]))),
    );
    assert_eq!(dim(&state, "physical"), 0.0);
}

/// TC-U06 · ported from tests/test_domain.py::test_san_clamps_at_100
#[test]
fn test_san_clamps_at_100() {
    let mut state = initial_state();
    apply_event(&mut state, &ev(make_task_created("t1")));
    apply_event(
        &mut state,
        &ev(make_task_completed("t1", json!([{"dimension": "san", "delta": 30.0}]))),
    );
    assert_eq!(dim(&state, "san"), 100.0);
}

/// TC-U07 · ported from tests/test_domain.py::test_repeatable_task_stays_active
#[test]
fn test_repeatable_task_stays_active() {
    let mut state = initial_state();
    let mut created = make_task_created("t1");
    created["repeatable"] = json!(true);
    apply_event(&mut state, &ev(created));
    apply_event(&mut state, &ev(make_task_completed("t1", json!([{"dimension": "knowledge", "delta": 5.0}]))));
    apply_event(&mut state, &ev(make_task_completed("t1", json!([{"dimension": "knowledge", "delta": 5.0}]))));
    let task = state.tasks.get("t1").unwrap();
    assert_eq!(task.status, "active");
    assert_eq!(task.times_completed, 2);
    assert_eq!(dim(&state, "knowledge"), 10.0);
}

/// TC-U08 · ported from tests/test_domain.py::test_completed_event_for_unknown_task_ignored
#[test]
fn test_completed_event_for_unknown_task_ignored() {
    let mut state = initial_state();
    apply_event(&mut state, &ev(make_task_completed("ghost", json!([{"dimension": "knowledge", "delta": 5.0}]))));
    assert!(!state.tasks.contains_key("ghost"));
    assert_eq!(dim(&state, "knowledge"), 0.0);
    assert!(state.logs.is_empty());
}

/// TC-U09 · ported from tests/test_domain.py::test_completed_event_for_deleted_task_ignored
#[test]
fn test_completed_event_for_deleted_task_ignored() {
    let mut state = initial_state();
    apply_event(&mut state, &ev(make_task_created("t1")));
    apply_event(
        &mut state,
        &ev(json!({"type": "TASK_DELETED", "id": "t1", "deleted": true, "deleted_at": "x", "date": "2026-09-19"})),
    );
    apply_event(&mut state, &ev(make_task_completed("t1", json!([{"dimension": "knowledge", "delta": 5.0}]))));
    assert_eq!(dim(&state, "knowledge"), 0.0);
}

/// TC-U10 · ported from tests/test_domain.py::test_task_updated_revives_completed_once_repeatable
#[test]
fn test_task_updated_revives_completed_once_repeatable() {
    let mut state = initial_state();
    apply_event(&mut state, &ev(make_task_created("t1")));
    apply_event(&mut state, &ev(make_task_completed("t1", json!([{"dimension": "knowledge", "delta": 5.0}]))));
    apply_event(
        &mut state,
        &ev(json!({
            "type": "TASK_UPDATED",
            "id": "t1",
            "title": "t1v2",
            "changes": [],
            "effects": [{"dimension": "knowledge", "delta": 8.0}],
            "repeatable": true,
            "date": "2026-09-19",
        })),
    );
    let task = state.tasks.get("t1").unwrap();
    assert_eq!(task.title, "t1v2");
    assert_eq!(task.status, "active");
    assert_eq!(task.completed_at, None);
    assert_eq!(task.effects[0].delta, 8.0);
}

/// TC-U11 · ported from tests/test_domain.py::test_task_deleted_removes_from_epic
#[test]
fn test_task_deleted_removes_from_epic() {
    let mut state = initial_state();
    apply_event(&mut state, &ev(make_epic_created("e1")));
    let mut created = make_task_created("t1");
    created["epic_id"] = json!("e1");
    apply_event(&mut state, &ev(created));
    apply_event(
        &mut state,
        &ev(json!({"type": "TASK_DELETED", "id": "t1", "deleted": true, "deleted_at": "x", "date": "2026-09-19"})),
    );
    let task = state.tasks.get("t1").unwrap();
    assert!(task.deleted);
    assert_eq!(task.status, "deleted");
    assert!(!state.epics.get("e1").unwrap().task_ids.contains(&"t1".to_string()));
}

// ---------------------------------------------------------------------------
// DailyTickTests
// ---------------------------------------------------------------------------

/// TC-U12 · ported from tests/test_domain.py::test_tick_records_history_and_resets_san
/// （ERR-001 修复版：显式钉死 current_day，断言语义与 Python 确定性版一致）
#[test]
fn test_tick_records_history_and_resets_san() {
    let mut state = initial_state();
    // Python 修复版在 initial_state 后覆写 current_day；Rust 版直接构造同一起点。
    state.meta.current_day = Some("2026-09-19".to_string());
    apply_event(&mut state, &ev(make_task_created("t1")));
    apply_event(
        &mut state,
        &ev(make_task_completed("t1", json!([{"dimension": "san", "delta": -30.0}]))),
    );
    assert_eq!(dim(&state, "san"), 70.0);
    apply_event(&mut state, &ev(json!({"type": "SYSTEM_DAILY_TICK", "date": "2026-09-20", "changes": []})));
    assert_eq!(dim(&state, "san"), 100.0);
    assert_eq!(
        state.meta.daily_san_history.get("2026-09-19").and_then(|value| value.as_f64()),
        Some(70.0)
    );
    assert_eq!(state.meta.last_tick_day.as_deref(), Some("2026-09-20"));
}

// ---------------------------------------------------------------------------
// EpicAndTitleTests
// ---------------------------------------------------------------------------

/// TC-U13 · ported from tests/test_domain.py::test_epic_created_builds_paired_title
#[test]
fn test_epic_created_builds_paired_title() {
    let mut state = initial_state();
    let mut created = make_epic_created("e1");
    created["title_bonus_percent"] = json!(15);
    apply_event(&mut state, &ev(created));
    let title = state.titles.get("e1").unwrap();
    assert!(!title.unlocked);
    assert_eq!(title.bonus_percent, 15.0);
    assert_eq!(title.target_dimension, "professional");
}

/// TC-U14 · ported from tests/test_domain.py::test_invalid_bonus_dimension_falls_back
#[test]
fn test_invalid_bonus_dimension_falls_back() {
    let mut state = initial_state();
    let mut created = make_epic_created("e1");
    created["title_bonus_dimension"] = json!("san");
    apply_event(&mut state, &ev(created));
    assert_eq!(state.titles.get("e1").unwrap().target_dimension, "professional");
    let mut created2 = make_epic_created("e2");
    created2["title_bonus_percent"] = json!("not-a-number");
    apply_event(&mut state, &ev(created2));
    assert_eq!(state.titles.get("e2").unwrap().bonus_percent, 0.0);
}

/// TC-U15 · ported from tests/test_domain.py::test_epic_completed_unlocks_title
#[test]
fn test_epic_completed_unlocks_title() {
    let mut state = initial_state();
    apply_event(&mut state, &ev(make_epic_created("e1")));
    apply_event(
        &mut state,
        &ev(json!({"type": "EPIC_COMPLETED", "id": "e1", "engraving": "刻下印记", "date": "2026-09-19", "changes": []})),
    );
    assert_eq!(state.epics.get("e1").unwrap().status, "completed");
    assert!(state.titles.get("e1").unwrap().unlocked);
    assert_eq!(state.epics.get("e1").unwrap().engraving, "刻下印记");
}

/// TC-U16 · ported from tests/test_domain.py::test_equip_max_three_enforced_in_reducer
#[test]
fn test_equip_max_three_enforced_in_reducer() {
    let mut state = initial_state();
    for index in 0..4 {
        apply_event(&mut state, &ev(make_epic_created(&format!("e{index}"))));
        apply_event(
            &mut state,
            &ev(json!({"type": "EPIC_COMPLETED", "id": format!("e{index}"), "engraving": "x", "date": "2026-09-19", "changes": []})),
        );
    }
    for index in 0..4 {
        apply_event(&mut state, &ev(json!({"type": "TITLE_EQUIPPED", "title_id": format!("e{index}"), "changes": []})));
    }
    assert_eq!(state.equipped.len(), 3);
    assert!(!state.equipped.contains(&"e3".to_string()));
}

/// TC-U17 · ported from tests/test_domain.py::test_unequip_removes
#[test]
fn test_unequip_removes() {
    let mut state = initial_state();
    apply_event(&mut state, &ev(make_epic_created("e1")));
    apply_event(
        &mut state,
        &ev(json!({"type": "EPIC_COMPLETED", "id": "e1", "engraving": "x", "date": "2026-09-19", "changes": []})),
    );
    apply_event(&mut state, &ev(json!({"type": "TITLE_EQUIPPED", "title_id": "e1", "changes": []})));
    apply_event(&mut state, &ev(json!({"type": "TITLE_UNEQUIPPED", "title_id": "e1", "changes": []})));
    assert!(state.equipped.is_empty());
}

/// TC-U18 · ported from tests/test_domain.py::test_locked_title_cannot_equip
#[test]
fn test_locked_title_cannot_equip() {
    let mut state = initial_state();
    apply_event(&mut state, &ev(make_epic_created("e1")));
    apply_event(&mut state, &ev(json!({"type": "TITLE_EQUIPPED", "title_id": "e1", "changes": []})));
    assert!(state.equipped.is_empty());
}

// ---------------------------------------------------------------------------
// TitleBonusTests
// ---------------------------------------------------------------------------

fn state_with_titles() -> State {
    let mut state = initial_state();
    for (index, percent) in [10.0, 25.0].iter().enumerate() {
        let mut created = make_epic_created(&format!("e{index}"));
        created["title_bonus_percent"] = json!(percent);
        apply_event(&mut state, &ev(created));
        apply_event(
            &mut state,
            &ev(json!({"type": "EPIC_COMPLETED", "id": format!("e{index}"), "engraving": "x", "date": "2026-09-19", "changes": []})),
        );
        apply_event(&mut state, &ev(json!({"type": "TITLE_EQUIPPED", "title_id": format!("e{index}"), "changes": []})));
    }
    state
}

/// TC-U19 · ported from tests/test_domain.py::test_bonuses_stack_additively
#[test]
fn test_bonuses_stack_additively() {
    let state = state_with_titles();
    let effects = apply_title_bonuses(&[fx("professional", 100.0)], &state);
    assert_eq!(effects[0].delta, 135.0);
    assert_eq!(effects[0].bonus_percent, Some(35.0));
}

/// TC-U20 · ported from tests/test_domain.py::test_zero_delta_dropped_and_unknown_dimension_ignored
#[test]
fn test_zero_delta_dropped_and_unknown_dimension_ignored() {
    let state = state_with_titles();
    let effects = apply_title_bonuses(
        &[fx("professional", 0.0), fx("willpower", 5.0), fx("san", 5.0)],
        &state,
    );
    assert_eq!(effects.len(), 1);
    assert_eq!(effects[0].dimension, "san");
}

/// TC-U21 · ported from tests/test_domain.py::test_negative_delta_amplified_by_bonus
#[test]
fn test_negative_delta_amplified_by_bonus() {
    let state = state_with_titles();
    let effects = apply_title_bonuses(&[fx("professional", -10.0)], &state);
    assert_eq!(effects[0].delta, -13.5);
}

// ---------------------------------------------------------------------------
// LegacyEventTests
// ---------------------------------------------------------------------------

/// TC-U22 · ported from tests/test_domain.py::test_legacy_single_dimension_event
#[test]
fn test_legacy_single_dimension_event() {
    let mut state = initial_state();
    apply_event(
        &mut state,
        &ev(json!({
            "type": "TASK_CREATED",
            "id": "t1",
            "title": "旧版",
            "date": "2026-09-19",
            "dimension": "knowledge",
            "exp_delta": 7,
            "san_delta": -3,
        })),
    );
    let task = state.tasks.get("t1").unwrap();
    assert_eq!(task.effects.len(), 2);
    assert_eq!(task.effects[0].dimension, "knowledge");
    assert_eq!(task.effects[0].delta, 7.0);
    assert_eq!(task.effects[1].dimension, "san");
    assert_eq!(task.effects[1].delta, -3.0);
}

/// TC-U23 · ported from tests/test_domain.py::test_unknown_event_type_ignored
#[test]
fn test_unknown_event_type_ignored() {
    let mut state = initial_state();
    apply_event(&mut state, &ev(json!({"type": "SOMETHING_NEW", "date": "2026-09-19"})));
    let fresh = initial_state();
    // 未知事件不得改变任何状态（时钟字段除外——空流场景锚定当天，两侧同为"今天"）。
    assert_eq!(state.tasks, fresh.tasks);
    assert_eq!(state.dimensions, fresh.dimensions);
    assert_eq!(state.epics, fresh.epics);
    assert_eq!(state.titles, fresh.titles);
    assert!(state.logs.is_empty());
}

// ---------------------------------------------------------------------------
// BuildStateTests
// ---------------------------------------------------------------------------

/// TC-U24 · ported from tests/test_domain.py::test_event_deleted_and_stream_cleared_skipped_in_replay
#[test]
fn test_event_deleted_and_stream_cleared_skipped_in_replay() {
    let events = [
        ev(json!({"type": "SYSTEM_INIT", "date": "2026-09-19", "changes": []})),
        ev(make_task_created("t1")),
        ev(json!({"type": "EVENT_DELETED", "target_event_id": "x", "changes": []})),
        ev(json!({"type": "STREAM_CLEARED", "changes": []})),
    ];
    let state = build_state(&events);
    assert!(state.tasks.contains_key("t1"));
    let types = state.event_stream.iter().map(|item| item.event_type.as_str()).collect::<Vec<_>>();
    assert!(!types.contains(&"EVENT_DELETED"));
    assert!(!types.contains(&"STREAM_CLEARED"));
}

/// TC-U25 · ported from tests/test_domain.py::test_event_stream_marks_tombstoned_event
#[test]
fn test_event_stream_marks_tombstoned_event() {
    let events = [
        ev(json!({"type": "SYSTEM_INIT", "date": "2026-09-19", "changes": [], "event_id": "ev1"})),
        {
            let mut created = make_task_created("t1");
            created["event_id"] = json!("ev2");
            ev(created)
        },
        ev(json!({"type": "EVENT_DELETED", "target_event_id": "ev2", "deleted_at": "2026-09-19T11:00:00", "changes": []})),
    ];
    let state = build_state(&events);
    let target = state
        .event_stream
        .iter()
        .find(|item| item.event_id.as_deref() == Some("ev2"))
        .unwrap();
    assert!(target.deleted);
    assert_eq!(target.deleted_at.as_deref(), Some("2026-09-19T11:00:00"));
}

/// TC-U26 · ported from tests/test_domain.py::test_derived_san_fields
#[test]
fn test_derived_san_fields() {
    let events = [
        ev(json!({"type": "SYSTEM_INIT", "date": "2026-09-19", "changes": []})),
        ev(make_task_created("t1")),
        ev(make_task_completed("t1", json!([{"dimension": "san", "delta": -20.0}]))),
    ];
    let state = build_state(&events);
    assert_eq!(state.derived.san, 80.0);
    assert_eq!(state.derived.san_change, -20.0);
    assert_eq!(state.daily_san.current, 80.0);
}

// ---------------------------------------------------------------------------
// Rust 侧新增护栏
// ---------------------------------------------------------------------------

/// TC-U45：14 种事件 serde 往返字节等价（以 golden 构造序列资产为夹具）。
#[test]
fn test_event_serde_roundtrip_all_types() {
    let asset = include_str!("assets/golden_synthetic_events.jsonl");
    let mut types_seen = std::collections::BTreeSet::new();
    for line in asset.lines().filter(|line| !line.trim().is_empty()) {
        let original: Value = serde_json::from_str(line).expect("资产行必须是合法 JSON");
        let event = Event::from_value(original.clone());
        let roundtrip = serde_json::to_value(&event).expect("事件必须可序列化");
        assert_eq!(roundtrip, original, "往返不得丢失/改写任何字段: {line}");
        types_seen.insert(event.type_tag().to_string());
    }
    assert_eq!(types_seen.len(), 14, "构造序列必须覆盖全部 14 种事件类型，实际: {types_seen:?}");
}

/// TC-U46：未知 type tag 落入 Unknown 兜底（对齐 Python 未知类型忽略语义）。
#[test]
fn test_unknown_tag_falls_to_unknown_variant() {
    let event = Event::from_value(json!({"type": "TOTALLY_NEW", "date": "2026-09-19", "changes": []}));
    assert!(matches!(event, Event::Unknown { .. }));
    assert_eq!(event.type_tag(), "TOTALLY_NEW");
}

/// TC-U47：SYSTEM_INIT 把 current_day 钉死为事件的 date 字段
/// （Python 现行为：current_day = profile.created_at = 事件 date，非时间戳）。
#[test]
fn test_system_init_pins_current_day_to_event_date() {
    let state = build_state(&[ev(json!({
        "type": "SYSTEM_INIT", "date": "2026-09-19", "changes": [],
        "at": "2026-09-19T14:59:09+08:00",
    }))]);
    assert_eq!(state.meta.current_day.as_deref(), Some("2026-09-19"));
    assert_eq!(state.profile.created_at.as_deref(), Some("2026-09-19"));
    assert_eq!(state.daily_san.day.as_deref(), Some("2026-09-19"));
}

/// TC-U48：同日 Tick 幂等守卫——tick.date == current_day 时历史不写入
/// （ERR-001 分析中发现的关键行为，Python 版必须原样保留）。
#[test]
fn test_same_day_tick_writes_no_history() {
    let mut state = initial_state();
    state.meta.current_day = Some("2026-09-20".to_string());
    apply_event(&mut state, &ev(json!({"type": "SYSTEM_DAILY_TICK", "date": "2026-09-20", "changes": []})));
    assert!(state.meta.daily_san_history.is_empty());
    assert_eq!(state.meta.last_tick_day.as_deref(), Some("2026-09-20"));
    assert_eq!(dim(&state, "san"), 100.0);
}

/// TC-U49：重放确定性守门——纯重放不读时钟，跨秒两次重放归一化结果相同。
#[test]
fn test_replay_is_time_independent() {
    let asset = include_str!("assets/golden_synthetic_events.jsonl");
    let events: Vec<Event> = asset
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| Event::from_value(serde_json::from_str(line).expect("合法 JSON")))
        .collect();
    let first = canonical_state_json(&build_state(&events));
    sleep(Duration::from_millis(1100)); // 跨过至少一个整秒
    let second = canonical_state_json(&build_state(&events));
    assert_eq!(first, second);
}
