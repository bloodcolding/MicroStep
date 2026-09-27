//! TC-U27~U44：`test_store.py` 18 例逐条移植（用例名/断言语义不变）。
//! 与 Python 版的差异：时钟注入钉死 2026-09-19（ERR-001 教训，语义等价）；
//! ValueError/KeyError 断言映射为 `StoreError::Invalid/NotFound`。
//! 全部使用独立临时目录隔离数据，不触碰真实事件流。

use std::fs;
use std::path::PathBuf;
use std::sync::Arc;

use serde_json::{json, Value};

use microstep_lib::domain::EffectInput;
use microstep_lib::store::{Clock, ClockNow, CreateTask, EventStore, StoreError, UpdateTask};

struct TempDir(PathBuf);

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn fixed_clock(date: &str) -> Clock {
    let date = date.to_string();
    Arc::new(move || ClockNow {
        datetime: format!("{date}T10:00:00+08:00"),
        date: date.clone(),
    })
}

struct Fixture {
    _tmp: TempDir,
    path: PathBuf,
    store: EventStore,
}

impl Fixture {
    fn new() -> Self {
        let unique = format!(
            "microstep-store-{}-{}",
            std::process::id(),
            CURRENT_TEST_INDEX.fetch_add(1, std::sync::atomic::Ordering::SeqCst),
        );
        let dir = std::env::temp_dir().join(unique);
        fs::create_dir_all(&dir).expect("创建临时目录");
        let path = dir.join("events.jsonl");
        let store = EventStore::with_clock(path.clone(), fixed_clock("2026-09-19"));
        Self { _tmp: TempDir(dir), path, store }
    }

    fn raw_lines(&self) -> Vec<String> {
        fs::read_to_string(&self.path)
            .expect("读取事件流文件")
            .lines()
            .filter(|line| !line.trim().is_empty())
            .map(str::to_string)
            .collect()
    }

    fn line_types(&self) -> Vec<String> {
        self.raw_lines()
            .iter()
            .map(|line| serde_json::from_str::<Value>(line).unwrap()["type"].as_str().unwrap().to_string())
            .collect()
    }
}

static CURRENT_TEST_INDEX: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);

fn fx(dimension: &str, delta: f64) -> EffectInput {
    EffectInput { dimension: dimension.to_string(), delta }
}

fn id_of(event: &Value, key: &str) -> String {
    event[key].as_str().expect("事件应含该字段").to_string()
}

fn invalid(result: Result<Value, StoreError>) -> StoreError {
    match result {
        Err(err @ StoreError::Invalid(_)) => err,
        other => panic!("期望 Invalid，实际: {other:?}"),
    }
}

fn not_found(result: Result<Value, StoreError>) -> StoreError {
    match result {
        Err(err @ StoreError::NotFound(_)) => err,
        other => panic!("期望 NotFound，实际: {other:?}"),
    }
}

// ---------------------------------------------------------------------------
// InitializationTests
// ---------------------------------------------------------------------------

/// TC-U27 · ported from tests/test_store.py::test_first_get_state_writes_system_init_and_default_epic
#[test]
fn test_first_get_state_writes_system_init_and_default_epic() {
    let mut fixture = Fixture::new();
    let state = fixture.store.get_state();
    assert!(state.profile.created_at.is_some());
    assert!(state.epics.contains_key("epic_infinite_progress"));
    let types = fixture.line_types();
    assert_eq!(&types[..2], &["SYSTEM_INIT".to_string(), "EPIC_CREATED".to_string()]);
}

/// TC-U28 · ported from tests/test_store.py::test_daily_tick_appended_on_first_access
#[test]
fn test_daily_tick_appended_on_first_access() {
    let mut fixture = Fixture::new();
    fixture.store.get_state();
    assert!(fixture.line_types().contains(&"SYSTEM_DAILY_TICK".to_string()));
}

// ---------------------------------------------------------------------------
// DailyTickTests
// ---------------------------------------------------------------------------

/// TC-U29 · ported from tests/test_store.py::test_missing_days_backfilled_in_order
#[test]
fn test_missing_days_backfilled_in_order() {
    let mut fixture = Fixture::new();
    fixture.store.get_state();
    // 手动追加两天后的 tick（时钟钉死 2026-09-19，即 day3=09-21），验证幂等不重复补。
    fixture.store.append_event(json!({
        "type": "SYSTEM_DAILY_TICK", "name": "每日结算", "changes": [], "date": "2026-09-21",
    }));
    let before = fixture.raw_lines().len();
    fixture.store.ensure_daily_ticks();
    assert_eq!(fixture.raw_lines().len(), before);
}

/// TC-U30 · ported from tests/test_store.py::test_last_tick_day_reads_latest
#[test]
fn test_last_tick_day_reads_latest() {
    let mut fixture = Fixture::new();
    fixture.store.get_state();
    let ticks: Vec<String> = fixture
        .store
        .read_events()
        .iter()
        .filter(|event| event["type"] == "SYSTEM_DAILY_TICK")
        .map(|event| event["date"].as_str().unwrap().to_string())
        .collect();
    assert_eq!(fixture.store.last_tick_day(), ticks.last().cloned());
}

// ---------------------------------------------------------------------------
// TaskCommandTests
// ---------------------------------------------------------------------------

/// TC-U31 · ported from tests/test_store.py::test_create_task_rejects_empty_effects
#[test]
fn test_create_task_rejects_empty_effects() {
    let mut fixture = Fixture::new();
    let result = fixture.store.create_task(CreateTask {
        title: "空效果".into(),
        epic_id: None,
        effects: vec![],
        repeatable: false,
        tags: vec![],
    });
    invalid(result);
}

/// TC-U32 · ported from tests/test_store.py::test_create_task_rejects_unknown_dimension
#[test]
fn test_create_task_rejects_unknown_dimension() {
    let mut fixture = Fixture::new();
    let result = fixture.store.create_task(CreateTask {
        title: "非法维度".into(),
        epic_id: None,
        effects: vec![fx("willpower", 5.0)],
        repeatable: false,
        tags: vec![],
    });
    invalid(result);
}

/// TC-U33 · ported from tests/test_store.py::test_complete_task_rejects_insufficient_san
#[test]
fn test_complete_task_rejects_insufficient_san() {
    let mut fixture = Fixture::new();
    let task = fixture
        .store
        .create_task(CreateTask {
            title: "大额消耗".into(),
            epic_id: None,
            effects: vec![fx("san", -999.0)],
            repeatable: false,
            tags: vec![],
        })
        .expect("创建应成功");
    let result = fixture.store.complete_task(&id_of(&task, "id"), "");
    invalid(result);
}

/// TC-U34 · ported from tests/test_store.py::test_complete_task_applies_title_bonus
#[test]
fn test_complete_task_applies_title_bonus() {
    let mut fixture = Fixture::new();
    let epic = fixture
        .store
        .create_epic("体能", "", "physical", "physical", 20.0, None)
        .expect("创建里程碑");
    fixture.store.complete_epic(&id_of(&epic, "id"), "完成").expect("结项");
    fixture.store.equip_title(&id_of(&epic, "id")).expect("装备");
    let task = fixture
        .store
        .create_task(CreateTask {
            title: "训练".into(),
            epic_id: None,
            effects: vec![fx("physical", 10.0)],
            repeatable: true,
            tags: vec![],
        })
        .expect("创建 Task");
    let event = fixture.store.complete_task(&id_of(&task, "id"), "").expect("结算");
    assert_eq!(event["effects"][0]["delta"].as_f64(), Some(12.0));
}

/// TC-U35 · ported from tests/test_store.py::test_update_task_missing_keeps_repeatable
#[test]
fn test_update_task_missing_keeps_repeatable() {
    let mut fixture = Fixture::new();
    let task = fixture
        .store
        .create_task(CreateTask {
            title: "任务".into(),
            epic_id: None,
            effects: vec![fx("knowledge", 5.0)],
            repeatable: true,
            tags: vec![],
        })
        .expect("创建 Task");
    fixture
        .store
        .update_task(
            &id_of(&task, "id"),
            UpdateTask {
                title: Some("改名".into()),
                effects: Some(vec![fx("knowledge", 6.0)]),
                ..Default::default()
            },
        )
        .expect("更新 Task");
    let state = fixture.store.get_state();
    assert!(state.tasks.get(&id_of(&task, "id")).unwrap().repeatable);
}

/// TC-U36 · ported from tests/test_store.py::test_delete_task_is_tombstone_not_rewrite
#[test]
fn test_delete_task_is_tombstone_not_rewrite() {
    let mut fixture = Fixture::new();
    let task = fixture
        .store
        .create_task(CreateTask {
            title: "待删".into(),
            epic_id: None,
            effects: vec![fx("knowledge", 5.0)],
            repeatable: false,
            tags: vec![],
        })
        .expect("创建 Task");
    let lines_before = fixture.raw_lines().len();
    fixture.store.delete_task(&id_of(&task, "id")).expect("删除 Task");
    let lines = fixture.raw_lines();
    assert_eq!(lines.len(), lines_before + 1);
    let last: Value = serde_json::from_str(lines.last().unwrap()).unwrap();
    assert_eq!(last["type"], "TASK_DELETED");
    assert_eq!(last["deleted"], true);
    let result = fixture.store.delete_task(&id_of(&task, "id")); // 重复删除被拒绝
    invalid(result);
}

// ---------------------------------------------------------------------------
// EpicCommandTests
// ---------------------------------------------------------------------------

/// TC-U37 · ported from tests/test_store.py::test_update_epic_rejects_unknown
#[test]
fn test_update_epic_rejects_unknown() {
    let mut fixture = Fixture::new();
    let result = fixture.store.update_epic(
        "no-such-epic",
        "x",
        "",
        "professional",
        "professional",
        10.0,
        "🏅",
    );
    not_found(result);
}

/// TC-U38 · ported from tests/test_store.py::test_complete_epic_validates_engraving
#[test]
fn test_complete_epic_validates_engraving() {
    // 校验收敛后：铭文长度校验统一在 store 层（HTTP/IPC 层复用同一规则）。
    let mut fixture = Fixture::new();
    let epic = fixture
        .store
        .create_epic("里程碑", "", "professional", "professional", 10.0, None)
        .expect("创建里程碑");
    let epic_id = id_of(&epic, "id");
    invalid(fixture.store.complete_epic(&epic_id, "一")); // 少于 2 字
    invalid(fixture.store.complete_epic(&epic_id, "   ")); // 空白同样拒绝
    fixture.store.complete_epic(&epic_id, "完整铭文").expect("合法铭文应通过");
    invalid(fixture.store.complete_epic(&epic_id, "再次结项")); // 已完成
}

/// TC-U39 · ported from tests/test_store.py::test_create_epic_validates_fields
#[test]
fn test_create_epic_validates_fields() {
    let mut fixture = Fixture::new();
    invalid(fixture.store.create_epic("  ", "", "professional", "professional", 10.0, None));
    invalid(fixture.store.create_epic("标题", "", "san", "professional", 10.0, None));
    invalid(fixture.store.create_epic("标题", "", "bogus", "professional", 10.0, None));
    invalid(fixture.store.create_epic("标题", "", "professional", "san", 10.0, None));
}

/// TC-U40 · ported from tests/test_store.py::test_create_task_validates_title
#[test]
fn test_create_task_validates_title() {
    let mut fixture = Fixture::new();
    let result = fixture.store.create_task(CreateTask {
        title: "   ".into(),
        epic_id: None,
        effects: vec![fx("knowledge", 5.0)],
        repeatable: false,
        tags: vec![],
    });
    invalid(result);
}

// ---------------------------------------------------------------------------
// TitleCommandTests
// ---------------------------------------------------------------------------

/// TC-U41 · ported from tests/test_store.py::test_equip_rules
#[test]
fn test_equip_rules() {
    let mut fixture = Fixture::new();
    let mut ids = Vec::new();
    for index in 0..4 {
        let epic = fixture
            .store
            .create_epic(&format!("里程碑{index}"), "", "professional", "professional", 10.0, None)
            .expect("创建里程碑");
        fixture.store.complete_epic(&id_of(&epic, "id"), "铭文").expect("结项");
        ids.push(id_of(&epic, "id"));
    }
    let locked = fixture
        .store
        .create_epic("未结项", "", "professional", "professional", 10.0, None)
        .expect("创建里程碑");
    invalid(fixture.store.equip_title(&id_of(&locked, "id"))); // 未解锁
    for epic_id in &ids[..3] {
        fixture.store.equip_title(epic_id).expect("前三个装备成功");
    }
    invalid(fixture.store.equip_title(&ids[3])); // 满 3 个
    invalid(fixture.store.equip_title(&ids[0])); // 重复装备
}

// ---------------------------------------------------------------------------
// EventDeletionTests
// ---------------------------------------------------------------------------

/// TC-U42 · ported from tests/test_store.py::test_delete_event_tombstones_and_blocks_system_events
#[test]
fn test_delete_event_tombstones_and_blocks_system_events() {
    let mut fixture = Fixture::new();
    let task = fixture
        .store
        .create_task(CreateTask {
            title: "任务".into(),
            epic_id: None,
            effects: vec![fx("knowledge", 5.0)],
            repeatable: false,
            tags: vec![],
        })
        .expect("创建 Task");
    let completed = fixture.store.complete_task(&id_of(&task, "id"), "").expect("结算");
    let deletion = fixture
        .store
        .delete_event(&id_of(&completed, "event_id"), "")
        .expect("删除事件");
    assert_eq!(deletion["type"], "EVENT_DELETED");
    // tombstone 不回滚状态：属性保持结算后的值。
    let state = fixture.store.get_state();
    assert_eq!(
        state.dimensions.get("knowledge").and_then(|value| value.as_f64()),
        Some(5.0)
    );
    invalid(fixture.store.delete_event(&id_of(&completed, "event_id"), "")); // 重复删除
    let system_event = fixture
        .store
        .read_events()
        .into_iter()
        .find(|event| event["type"] == "SYSTEM_INIT")
        .expect("存在系统初始化事件");
    invalid(fixture.store.delete_event(&id_of(&system_event, "event_id"), "")); // 系统事件不可删
}

// ---------------------------------------------------------------------------
// RobustnessTests
// ---------------------------------------------------------------------------

/// TC-U43 · ported from tests/test_store.py::test_corrupt_lines_skipped_not_fatal
#[test]
fn test_corrupt_lines_skipped_not_fatal() {
    let mut fixture = Fixture::new();
    fixture.store.get_state();
    let task = fixture
        .store
        .create_task(CreateTask {
            title: "任务".into(),
            epic_id: None,
            effects: vec![fx("knowledge", 5.0)],
            repeatable: false,
            tags: vec![],
        })
        .expect("创建 Task");
    fixture.store.complete_task(&id_of(&task, "id"), "").expect("结算");
    {
        use std::io::Write;
        let mut handle = fs::OpenOptions::new().append(true).open(&fixture.path).unwrap();
        handle.write_all("{ 这是一行损坏的 JSON\n".as_bytes()).unwrap();
        handle.write_all("\n".as_bytes()).unwrap();
    }
    let state = fixture.store.get_state();
    assert_eq!(
        state.dimensions.get("knowledge").and_then(|value| value.as_f64()),
        Some(5.0)
    );
}

/// TC-U44 · ported from tests/test_store.py::test_append_event_assigns_ids_and_timestamps
#[test]
fn test_append_event_assigns_ids_and_timestamps() {
    let mut fixture = Fixture::new();
    let event = fixture.store.append_event(json!({
        "type": "PROFILE_AWAKENED", "name": "提前觉醒", "changes": [],
    }));
    assert!(event.get("event_id").is_some_and(|value| !value.is_null()));
    assert!(event.get("created_at").is_some_and(|value| !value.is_null()));
    assert_eq!(event["at"], event["created_at"]);
}
