//! IPC command 层测试（TC-I01~I21 + TC-E14）：信封形状、校验拒绝路径与
//! Python 强制转换语义（Null→"None"、or 缺省、isinstance effects、
//! KeyError 带引号 error 文案）。语义承接 tests/test_server.py HTTP 24 例
//! 中可映射部分（tasks.md 3.3）；路由/静态资源/原始 JSON 解析属于 HTTP 层，不移植。

use std::path::PathBuf;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::{json, Value};

use microstep_lib::app_state::AppState;
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
    std::env::temp_dir().join(format!("microstep-ipc-{tag}-{nanos}"))
}

fn app_state(tag: &str) -> AppState {
    AppState::new(EventStore::with_clock(
        unique_dir(tag).join("events.jsonl"),
        fixed_clock("2026-01-05"),
    ))
}

fn task_body(overrides: Value) -> Value {
    // 与 tests/test_server.py::_create_task 同一基线请求体。
    let mut body = json!({
        "title": "测试 Task",
        "epic_id": "",
        "repeatable": false,
        "effects": [{"dimension": "knowledge", "delta": 5}],
    });
    if let (Some(target), Some(source)) = (body.as_object_mut(), overrides.as_object()) {
        for (key, value) in source {
            target.insert(key.clone(), value.clone());
        }
    }
    body
}

fn created_task_id(payload: &Value) -> String {
    payload["event"]["id"].as_str().unwrap().to_string()
}

// ---------------------------------------------------------------------------
// 读命令（GET 语义：原始 payload，无 ok 字段）
// ---------------------------------------------------------------------------

/// TC-I01 · ported from tests/test_server.py::test_state
#[test]
fn tc_i01_get_state_returns_raw_state() {
    let state = app_state("i01");
    let payload = state.get_state();
    assert!(payload.get("ok").is_none(), "GET /api/state 无信封包装: {payload}");
    assert!(payload["profile"]["created_at"].as_str().is_some_and(|v| !v.is_empty()));
    assert!(payload["epics"].as_object().is_some_and(|epics| epics.contains_key("epic_infinite_progress")));
}

/// TC-I02 · ported from tests/test_server.py::test_meta_contains_dimensions_and_slots
#[test]
fn tc_i02_get_meta_contains_dimensions_and_slots() {
    let payload = app_state("i02").get_meta();
    assert_eq!(payload["max_equipped_titles"], json!(3));
    assert!(payload["dimensions"].as_object().is_some_and(|d| d.contains_key("san")));
    assert!(payload["pool_dimensions"].as_array().is_some_and(|list| list.iter().any(|v| v == "physical")));
    assert!(payload["effect_dimensions"].as_array().is_some_and(|list| list.iter().any(|v| v == "san")));
}

// ---------------------------------------------------------------------------
// Task 链路（信封 + 校验拒绝）
// ---------------------------------------------------------------------------

/// TC-I03 · ported from tests/test_server.py::test_create_and_log_task
#[test]
fn tc_i03_create_and_log_task_envelope() {
    let state = app_state("i03");
    let created = state.create_task(&task_body(json!({})));
    assert_eq!(created["ok"], json!(true));
    assert!(created["event"].is_object());
    assert!(created["state"].is_object());
    let logged = state.log_task(&created_task_id(&created), &json!({"note": ""}));
    assert_eq!(logged["ok"], json!(true));
    assert_eq!(logged["state"]["dimensions"]["knowledge"].as_f64().unwrap(), 5.0);
}

/// TC-I04 · ported from tests/test_server.py::test_legacy_complete_alias
/// （/complete 别名与 /log 共用 log_task；空 body → note 缺省 ""）
#[test]
fn tc_i04_log_task_with_empty_body() {
    let state = app_state("i04");
    let created = state.create_task(&task_body(json!({"title": "旧接口"})));
    let logged = state.log_task(&created_task_id(&created), &json!({}));
    assert_eq!(logged["ok"], json!(true));
    assert_eq!(logged["event"]["note"], json!(""));
}

/// TC-I05 · ported from tests/test_server.py::test_create_task_rejects_empty_title
#[test]
fn tc_i05_create_task_rejects_empty_title() {
    let payload = app_state("i05").create_task(&task_body(json!({"title": "   "})));
    assert_eq!(payload["ok"], json!(false));
    assert!(payload["error"].as_str().unwrap().contains("标题"));
}

/// TC-I06 · ported from tests/test_server.py::test_create_task_rejects_bad_effects
#[test]
fn tc_i06_create_task_rejects_bad_effects() {
    let payload = app_state("i06").create_task(&task_body(json!({"effects": [{"dimension": "willpower", "delta": 5}]})));
    assert_eq!(payload["ok"], json!(false));
    assert_eq!(payload["error"], json!("Task 属性不合法"));
}

/// TC-I07 · ported from tests/test_server.py::test_log_task_rejects_insufficient_san
#[test]
fn tc_i07_log_task_rejects_insufficient_san() {
    let state = app_state("i07");
    let created = state.create_task(&task_body(json!({"effects": [{"dimension": "san", "delta": -999}]})));
    let logged = state.log_task(&created_task_id(&created), &json!({}));
    assert_eq!(logged["ok"], json!(false));
    assert!(logged["error"].as_str().unwrap().contains("SAN"));
}

/// TC-I08 · ported from tests/test_server.py::test_update_task
#[test]
fn tc_i08_update_task_applies_fields() {
    let state = app_state("i08");
    let created = state.create_task(&task_body(json!({})));
    let task_id = created_task_id(&created);
    let payload = state.update_task(
        &task_id,
        &json!({"title": "改名", "effects": [{"dimension": "knowledge", "delta": 8}], "repeatable": true}),
    );
    assert_eq!(payload["ok"], json!(true));
    let task = &payload["state"]["tasks"][&task_id];
    assert_eq!(task["title"], json!("改名"));
    assert_eq!(task["repeatable"], json!(true));
}

/// TC-I09 · ported from tests/test_server.py::test_delete_task_then_reject_duplicate
#[test]
fn tc_i09_delete_task_then_reject_duplicate() {
    let state = app_state("i09");
    let task_id = created_task_id(&state.create_task(&task_body(json!({}))));
    assert_eq!(state.delete_task(&task_id)["ok"], json!(true));
    let payload = state.delete_task(&task_id);
    assert_eq!(payload["ok"], json!(false));
    assert!(payload["error"].as_str().unwrap().contains("已删除"));
}

// ---------------------------------------------------------------------------
// Epic / 称号 / 系统
// ---------------------------------------------------------------------------

/// TC-I10 · ported from tests/test_server.py::test_create_and_complete_epic
#[test]
fn tc_i10_create_and_complete_epic() {
    let state = app_state("i10");
    let created = state.create_epic(&json!({"title": "测试里程碑", "title_bonus_percent": 10}));
    assert_eq!(created["ok"], json!(true));
    let epic_id = created["event"]["id"].as_str().unwrap();
    let completed = state.complete_epic(epic_id, &json!({"engraving": "完整铭文"}));
    assert_eq!(completed["ok"], json!(true));
    assert_eq!(completed["state"]["titles"][epic_id]["unlocked"], json!(true));
}

/// TC-I11 · ported from tests/test_server.py::test_complete_epic_rejects_short_engraving
#[test]
fn tc_i11_complete_epic_rejects_short_engraving() {
    let state = app_state("i11");
    let epic_id = state.create_epic(&json!({"title": "t"}))["event"]["id"].as_str().unwrap().to_string();
    let payload = state.complete_epic(&epic_id, &json!({"engraving": "短"}));
    assert_eq!(payload["ok"], json!(false));
    assert!(payload["error"].as_str().unwrap().contains("铭文"));
}

/// TC-I12 · ported from tests/test_server.py::test_create_epic_rejects_san_main_dimension
#[test]
fn tc_i12_create_epic_rejects_san_main_dimension() {
    let payload = app_state("i12").create_epic(&json!({"title": "t", "main_dimension": "san"}));
    assert_eq!(payload["ok"], json!(false));
    assert!(payload["error"].as_str().unwrap().contains("主维度"));
}

/// TC-I13 · ported from tests/test_server.py::test_update_epic（含 emoji 空白回退 🏅）
#[test]
fn tc_i13_update_epic_applies_fields() {
    let state = app_state("i13");
    let epic_id = state.create_epic(&json!({"title": "t"}))["event"]["id"].as_str().unwrap().to_string();
    let payload = state.update_epic(
        &epic_id,
        &json!({
            "title": "更新版",
            "description": "新描述",
            "main_dimension": "knowledge",
            "title_bonus_dimension": "knowledge",
            "title_bonus_percent": 15,
            "title_emoji": "   ",
        }),
    );
    assert_eq!(payload["ok"], json!(true));
    assert_eq!(payload["state"]["epics"][&epic_id]["title"], json!("更新版"));
    assert_eq!(payload["event"]["title_emoji"], json!("🏅"));
}

/// TC-I14 · ported from tests/test_server.py::test_equip_unequip_via_api
#[test]
fn tc_i14_equip_unequip_title() {
    let state = app_state("i14");
    let epic_id = state.create_epic(&json!({"title": "称号链路", "title_bonus_percent": 5}))["event"]["id"].as_str().unwrap().to_string();
    state.complete_epic(&epic_id, &json!({"engraving": "结项铭文"}));
    let equipped = state.equip_title(&json!({"title_id": epic_id}));
    assert_eq!(equipped["ok"], json!(true));
    assert!(equipped["state"]["equipped"].as_array().unwrap().iter().any(|v| v == epic_id.as_str()));
    let unequipped = state.unequip_title(&json!({"title_id": epic_id}));
    assert_eq!(unequipped["ok"], json!(true));
    assert!(!unequipped["state"]["equipped"].as_array().unwrap().iter().any(|v| v == epic_id.as_str()));
}

/// TC-I15 · ported from tests/test_server.py::test_awaken_once_only
#[test]
fn tc_i15_awaken_once_only() {
    let state = app_state("i15");
    assert_eq!(state.awaken()["ok"], json!(true));
    let payload = state.awaken();
    assert_eq!(payload["ok"], json!(false));
    assert!(payload["error"].as_str().unwrap().contains("觉醒"));
}

/// TC-I16 · ported from tests/test_server.py::test_system_tick
#[test]
fn tc_i16_system_tick_envelope() {
    let payload = app_state("i16").system_tick();
    assert_eq!(payload["ok"], json!(true));
    assert!(payload["state"].is_object());
}

/// TC-I17 · ported from tests/test_server.py::test_delete_event_via_api
#[test]
fn tc_i17_delete_event_tombstones_stream_item() {
    let state = app_state("i17");
    let task_id = created_task_id(&state.create_task(&task_body(json!({"effects": [{"dimension": "san", "delta": -1}]}))));
    let logged = state.log_task(&task_id, &json!({}));
    let event_id = logged["event"]["event_id"].as_str().unwrap();
    let payload = state.delete_event(event_id, &json!({"note": ""}));
    assert_eq!(payload["ok"], json!(true));
    let stream_item = payload["state"]["event_stream"]
        .as_array()
        .unwrap()
        .iter()
        .find(|item| item["event_id"] == event_id)
        .expect("事件流中存在目标事件");
    assert_eq!(stream_item["deleted"], json!(true));
}

// ---------------------------------------------------------------------------
// Python 强制转换语义（server.py 逐字段镜像）
// ---------------------------------------------------------------------------

/// TC-I18 · `str(body.get(k, ""))` 显式 null → "None" 怪癖。
#[test]
fn tc_i18_explicit_null_becomes_none_string() {
    let state = app_state("i18");
    let created = state.create_task(&task_body(json!({"title": null})));
    assert_eq!(created["ok"], json!(true));
    assert_eq!(created["event"]["title"], json!("None"));
    let logged = state.log_task(&created_task_id(&created), &json!({"note": null}));
    assert_eq!(logged["event"]["note"], json!("None"));
    let epic = state.create_epic(&json!({"title": "t", "description": null}));
    assert_eq!(epic["event"]["description"], json!("None"));
}

/// TC-I19 · `str(x or 默认)`：假值（""/0/false/null）回退默认；数字字符串可 float。
#[test]
fn tc_i19_or_fallback_coercion() {
    let state = app_state("i19");
    for bad in [json!(""), json!(0), json!(false)] {
        let payload = state.create_epic(&json!({"title": "t", "main_dimension": bad}));
        assert_eq!(payload["event"]["main_dimension"], json!("professional"), "假值 {bad} 应回退默认");
    }
    for bad in [json!(""), json!(0), json!(false), json!(null)] {
        let payload = state.create_epic(&json!({"title": "t", "title_bonus_percent": bad}));
        assert_eq!(payload["event"]["title_bonus_percent"].as_f64().unwrap(), 0.0, "假值 {bad} 应回退 0");
    }
    let text_percent = state.create_epic(&json!({"title": "t", "title_bonus_percent": "15"}));
    assert_eq!(text_percent["event"]["title_bonus_percent"].as_f64().unwrap(), 15.0);
    let no_emoji = state.create_epic(&json!({"title": "t"}));
    assert_eq!(no_emoji["event"]["title_emoji"], json!("🏅"));
}

/// TC-I20 · float 失败文案（ValueError 原文 / TypeError 内部错误前缀）
/// 与 KeyError→str 为 repr（error 带单引号）怪癖。
#[test]
fn tc_i20_float_and_keyerror_error_text() {
    let state = app_state("i20");
    let payload = state.create_epic(&json!({"title": "t", "title_bonus_percent": "abc"}));
    assert_eq!(payload["error"], json!("could not convert string to float: 'abc'"));
    let payload = state.create_epic(&json!({"title": "t", "title_bonus_percent": {"v": 1}}));
    assert_eq!(
        payload["error"],
        json!("服务器内部错误: float() argument must be a string or a real number, not 'dict'")
    );
    assert_eq!(state.delete_task("no-such-task")["error"], json!("'Task 不存在'"));
    assert_eq!(state.delete_event("no-such-event", &json!({}))["error"], json!("'事件不存在'"));
}

/// TC-I21 · effects 形状语义 / 校验顺序 / repeatable 与 epic_id 的缺省怪癖。
#[test]
fn tc_i21_effects_and_optional_field_semantics() {
    let state = app_state("i21");
    // 非列表 effects → 视为缺省（isinstance）→ 至少一个效果校验拒绝。
    for bad in [json!("nope"), json!({"a": 1}), json!(5)] {
        let payload = state.create_task(&task_body(json!({"effects": bad})));
        assert_eq!(payload["ok"], json!(false), "非列表 effects {bad} 应拒绝");
        assert_eq!(payload["error"], json!("至少设置一个属性增益或减益"));
    }
    // delta 数字字符串生效（Python float("5")）；dimension 校验先于 delta。
    let created = state.create_task(&task_body(json!({"effects": [{"dimension": "knowledge", "delta": "5"}]})));
    assert_eq!(created["ok"], json!(true));
    state.log_task(&created_task_id(&created), &json!({}));
    assert_eq!(state.get_state()["dimensions"]["knowledge"].as_f64().unwrap(), 5.0);
    let payload = state.create_task(&task_body(json!({"effects": [{"dimension": "willpower", "delta": "abc"}]})));
    assert_eq!(payload["error"], json!("Task 属性不合法"));
    // repeatable：键缺省 → 保持原值；显式 null → bool(None) = false。
    let task_id = created_task_id(&state.create_task(&task_body(json!({"repeatable": true}))));
    let kept = state.update_task(&task_id, &json!({"title": "r", "effects": [{"dimension": "knowledge", "delta": 5}]}));
    assert_eq!(kept["state"]["tasks"][&task_id]["repeatable"], json!(true));
    let nulled = state.update_task(
        &task_id,
        &json!({"title": "r", "effects": [{"dimension": "knowledge", "delta": 5}], "repeatable": null}),
    );
    assert_eq!(nulled["state"]["tasks"][&task_id]["repeatable"], json!(false));
    // epic_id：更新缺省 → _clean_id(None) → None → 移出里程碑。
    let epic_task = created_task_id(&state.create_task(&task_body(json!({"epic_id": "epic_infinite_progress"}))));
    let moved = state.update_task(&epic_task, &json!({"title": "r", "effects": [{"dimension": "knowledge", "delta": 5}]}));
    assert!(moved["state"]["tasks"][&epic_task]["epic_id"].is_null());
}

// ---------------------------------------------------------------------------
// 并发
// ---------------------------------------------------------------------------

/// TC-E14 · 八线程并发建 Task + 记录：共享 AppState（Mutex 串行化），
/// 信封全部 ok 且最终状态一致（等价 Python 多线程 HTTP 并发写事件流）。
#[test]
fn tc_e14_eight_threads_concurrent_commands() {
    let state = Arc::new(app_state("e14"));
    let mut handles = Vec::new();
    for i in 0..8 {
        let state = Arc::clone(&state);
        handles.push(std::thread::spawn(move || {
            let created = state.create_task(&json!({
                "title": format!("并发 Task {i}"),
                "epic_id": "",
                "repeatable": true,
                "effects": [{"dimension": "knowledge", "delta": 1}],
            }));
            assert_eq!(created["ok"], json!(true), "并发创建失败: {created}");
            let logged = state.log_task(&created_task_id(&created), &json!({}));
            assert_eq!(logged["ok"], json!(true), "并发记录失败: {logged}");
        }));
    }
    for handle in handles {
        handle.join().expect("工作线程不应 panic");
    }
    let payload = state.get_state();
    assert_eq!(payload["tasks"].as_object().unwrap().len(), 8);
    assert_eq!(payload["dimensions"]["knowledge"].as_f64().unwrap(), 8.0);
}
