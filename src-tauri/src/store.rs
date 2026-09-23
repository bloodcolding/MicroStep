//! JSONL 事件流存储与业务命令（移植自 `backend/store.py`，行为等价）。
//!
//! 与 Python 版的差异（结构性，语义等价）：
//! - 时钟**可注入**：Python 版直接读真实时钟（`iso_date`/`now_iso`），Rust 版以
//!   `Clock` 闭包注入，生产装配本地时区真实时钟（随 Tauri 壳批次落位），测试钉死
//!   固定日期（ERR-001 教训：时钟敏感逻辑必须可测）；`open` 暂留未实现占位；
//! - ValueError/KeyError → `StoreError::Invalid/NotFound`（command 层据此映射
//!   `{ok:false, error}` 信封文案）；
//! - event_id 以「纳秒时间戳 + 进程内单调计数」生成 36 位 uuid 形状（无 uuid crate
//!   依赖；Tauri 依赖树到位后如需可替换为 RFC 版本）；
//! - mtime 缓存：同一 mtime 只解析一次 JSONL；追加后缓存失效（外部改文件同理）。

use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::{json, Value};

use crate::domain::{
    apply_title_bonuses, build_state, days_from_date, date_from_days, has_dimension, round1,
    Effect, EffectInput, Event, State, POOL_DIMENSIONS,
};

/// 业务错误：Invalid ≈ Python ValueError（校验拒绝），NotFound ≈ KeyError（实体不存在）。
#[derive(Debug, Clone, PartialEq)]
pub enum StoreError {
    Invalid(String),
    NotFound(String),
}

/// 时钟快照：date 为本地自然日（YYYY-MM-DD），datetime 为带时区 ISO 时间戳。
#[derive(Debug, Clone)]
pub struct ClockNow {
    pub date: String,
    pub datetime: String,
}

/// 可注入时钟（生产 = 本地时区真实时钟；测试 = 固定日期）。
pub type Clock = Arc<dyn Fn() -> ClockNow + Send + Sync>;

/// 生产真实时钟：本地时区（等价 Python `date.today()` / `datetime.now().astimezone()`）。
pub fn real_clock() -> Clock {
    Arc::new(|| {
        let now = chrono::Local::now();
        ClockNow {
            date: now.date_naive().format("%Y-%m-%d").to_string(),
            datetime: now.format("%Y-%m-%dT%H:%M:%S%:z").to_string(),
        }
    })
}

/// `create_task` 参数（Python 位置参数 + kwargs 的等价收敛）。
#[derive(Debug, Clone, Default)]
pub struct CreateTask {
    pub title: String,
    pub epic_id: Option<String>,
    pub effects: Vec<EffectInput>,
    pub repeatable: bool,
    pub tags: Vec<String>,
}

/// `update_task` 参数。`None` 语义与 Python kwargs 缺省一致：
/// title/effects 为 None → Invalid（Python 空列表同样拒绝）；epic_id None → 移出里程碑；
/// repeatable None → 保持原值。
#[derive(Debug, Clone, Default)]
pub struct UpdateTask {
    pub title: Option<String>,
    pub epic_id: Option<String>,
    pub effects: Option<Vec<EffectInput>>,
    pub repeatable: Option<bool>,
}

static EVENT_ID_COUNTER: AtomicU64 = AtomicU64::new(0);

/// uuid4 形状事件 id：纳秒时间戳 + 进程内单调计数（同进程内保证唯一）。
fn new_event_id() -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos() as u64;
    let count = EVENT_ID_COUNTER.fetch_add(1, Ordering::Relaxed) as u64;
    let raw: u128 = ((nanos as u128) << 64) | (count as u128);
    let hex = format!("{raw:032x}");
    format!(
        "{}-{}-4{}-8{}-{}",
        &hex[0..8],
        &hex[8..12],
        &hex[13..16],
        &hex[17..20],
        &hex[20..32],
    )
}

/// Python `_normalize_effects`：白名单校验 + round1 + 同维度合并（保持首现顺序）+ 去零。
fn normalize_effects(effects: &[EffectInput]) -> Result<Vec<Effect>, StoreError> {
    let mut merged: Vec<(String, f64)> = Vec::new();
    for item in effects {
        if !has_dimension(&item.dimension) {
            return Err(StoreError::Invalid("Task 属性不合法".to_string()));
        }
        let delta = round1(item.delta);
        if delta == 0.0 {
            continue;
        }
        match merged.iter_mut().find(|(dimension, _)| *dimension == item.dimension) {
            Some(entry) => entry.1 = round1(entry.1 + delta),
            None => merged.push((item.dimension.clone(), delta)),
        }
    }
    Ok(merged
        .into_iter()
        .filter(|(_, delta)| *delta != 0.0)
        .map(|(dimension, delta)| Effect {
            dimension,
            delta,
            base_delta: None,
            bonus_percent: None,
        })
        .collect())
}

/// Python `_validate_epic_fields`（创建/更新共用的唯一校验点）。
fn validate_epic_fields(title: &str, main_dimension: &str, title_bonus_dimension: &str) -> Result<(), StoreError> {
    if title.trim().is_empty() {
        return Err(StoreError::Invalid("请填写 Epic 标题".to_string()));
    }
    if !has_dimension(main_dimension) || main_dimension == "san" {
        return Err(StoreError::Invalid("主维度不合法".to_string()));
    }
    if !POOL_DIMENSIONS.contains(&title_bonus_dimension) {
        return Err(StoreError::Invalid("称号加成属性不合法".to_string()));
    }
    Ok(())
}

/// Python `%g` 浮点文案（str(100.0) → "100.0"）。
pub(crate) fn fmt_f64(value: f64) -> String {
    if value.is_finite() && value == value.trunc() && value.abs() < 1e15 {
        format!("{value:.1}")
    } else {
        format!("{value}")
    }
}

/// 追加式 JSONL 事件流存储：所有状态经重放生成，事件不可变（tombstone 删除）。
pub struct EventStore {
    path: PathBuf,
    clock: Clock,
    events_cache: Option<Vec<Value>>,
    cache_mtime: Option<SystemTime>,
}

impl EventStore {
    /// 事件流文件路径（同步引擎据此定位数据目录）。
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// 以注入时钟打开（或创建）事件流文件（测试入口，时钟钉死）。
    pub fn with_clock(path: PathBuf, clock: Clock) -> Self {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).expect("创建事件流父目录");
        }
        if !path.exists() {
            File::create(&path).expect("创建事件流文件");
        }
        EventStore { path, clock, events_cache: None, cache_mtime: None }
    }

    /// 以生产真实时钟打开（本地时区；AppData 落盘由 Tauri 壳接线）。
    pub fn open(path: PathBuf) -> Self {
        EventStore::with_clock(path, real_clock())
    }

    fn clock_now(&self) -> ClockNow {
        (self.clock)()
    }

    fn today(&self) -> String {
        self.clock_now().date
    }

    fn now_iso(&self) -> String {
        self.clock_now().datetime
    }

    fn read_lines_raw(&self) -> Vec<String> {
        match fs::read_to_string(&self.path) {
            Ok(content) => content.lines().filter(|line| !line.trim().is_empty()).map(String::from).collect(),
            Err(_) => Vec::new(),
        }
    }

    fn mtime(&self) -> Option<SystemTime> {
        fs::metadata(&self.path).and_then(|meta| meta.modified()).ok()
    }

    /// 读取全部事件（损坏行容错跳过；mtime 未变走缓存）。
    pub fn read_events(&mut self) -> Vec<Value> {
        let mtime = self.mtime();
        if self.events_cache.is_none() || mtime != self.cache_mtime {
            let mut events = Vec::new();
            for line in self.read_lines_raw() {
                match serde_json::from_str::<Value>(&line) {
                    Ok(event) => events.push(event),
                    Err(_) => continue, // 单行损坏不阻断整个流。
                }
            }
            self.events_cache = Some(events);
            self.cache_mtime = mtime;
        }
        self.events_cache.clone().unwrap_or_default()
    }

    /// 追加事件：注入 event_id/created_at（缺省时）并落盘，返回补全后的事件。
    pub fn append_event(&mut self, mut event: Value) -> Value {
        let Value::Object(fields) = &mut event else {
            panic!("append_event 仅接受 JSON 对象事件");
        };
        if !fields.contains_key("event_id") {
            fields.insert("event_id".to_string(), Value::String(new_event_id()));
        }
        if !fields.contains_key("created_at") {
            let stamp = Value::String(self.now_iso());
            fields.insert("created_at".to_string(), stamp.clone());
            // Python：at 缺省回填 created_at（setdefault 语义）。
            fields.entry("at".to_string()).or_insert(stamp);
        } else if !fields.contains_key("at") {
            let stamp = fields.get("created_at").cloned().unwrap_or(Value::Null);
            fields.insert("at".to_string(), stamp);
        }
        let serialized = serde_json::to_string(&event).expect("事件序列化不可失败");
        let mut handle = OpenOptions::new().append(true).open(&self.path).expect("打开事件流文件");
        handle.write_all(serialized.as_bytes()).expect("写入事件流");
        handle.write_all(b"\n").expect("写入换行");
        // 追加后让缓存失效，下次读取重新解析（外部直接改文件同理靠 mtime 失效）。
        self.events_cache = None;
        event
    }

    /// 首次访问初始化：SYSTEM_INIT + 默认里程碑「无限进步」（已有内容则跳过）。
    pub fn ensure_initialized(&mut self) {
        if !self.read_lines_raw().is_empty() {
            return;
        }
        let today = self.today();
        self.append_event(json!({
            "type": "SYSTEM_INIT",
            "name": "系统初始化",
            "changes": [],
            "date": today,
            "at": self.now_iso(),
        }));
        self.append_event(json!({
            "type": "EPIC_CREATED",
            "id": "epic_infinite_progress",
            "name": "创建里程碑：无限进步",
            "changes": [],
            "title": "无限进步",
            "description": "系统默认里程碑。所有未明确归属的日常 Task 都会沉淀到这里。",
            "main_dimension": "professional",
            "title_emoji": "🚀",
            "title_bonus_dimension": "professional",
            "title_bonus_percent": 0,
            "unlock_title_id": null,
            "date": today,
            "at": self.now_iso(),
        }));
    }

    /// 补齐缺失的每日 Tick（幂等：最后 tick 已是今天则不补；日期非法则不写）。
    pub fn ensure_daily_ticks(&mut self) {
        self.ensure_initialized();
        let today = self.today();
        let last = self.last_tick_day();
        if last == Some(today.clone()) {
            return;
        }
        let start = match last {
            Some(day) => {
                let Some(days) = days_from_date(&day) else { return };
                days + 1
            }
            None => days_from_date(&today).unwrap_or(0),
        };
        let Some(end) = days_from_date(&today) else { return };
        let mut cursor = start;
        while cursor <= end {
            self.append_event(json!({
                "type": "SYSTEM_DAILY_TICK",
                "name": "每日结算",
                "changes": [],
                "date": date_from_days(cursor),
                "at": self.now_iso(),
            }));
            cursor += 1;
        }
    }

    /// 最后一条 Tick 的日期。
    pub fn last_tick_day(&mut self) -> Option<String> {
        self.read_events()
            .iter()
            .rev()
            .find(|event| event.get("type").and_then(Value::as_str) == Some("SYSTEM_DAILY_TICK"))
            .and_then(|event| event.get("date").and_then(Value::as_str).map(String::from))
    }

    /// 业务命令统一前置（初始化 + 补 Tick）。
    fn ensure_ready(&mut self) {
        self.ensure_initialized();
        self.ensure_daily_ticks();
    }

    /// 业务命令统一前置后重放全量状态。
    pub fn get_state(&mut self) -> State {
        self.ensure_ready();
        let events: Vec<Event> = self
            .read_events()
            .into_iter()
            .map(Event::from_value)
            .collect();
        build_state(&events)
    }

    pub fn create_epic(
        &mut self,
        title: &str,
        description: &str,
        main_dimension: &str,
        title_bonus_dimension: &str,
        title_bonus_percent: f64,
        title_emoji: Option<&str>,
    ) -> Result<Value, StoreError> {
        self.ensure_ready();
        validate_epic_fields(title, main_dimension, title_bonus_dimension)?;
        let event = json!({
            "type": "EPIC_CREATED",
            "id": new_event_id(),
            "name": format!("创建里程碑：{title}"),
            "changes": [],
            "title": title,
            "description": description,
            "main_dimension": main_dimension,
            "title_emoji": title_emoji.unwrap_or("🏅"),
            "title_bonus_dimension": title_bonus_dimension,
            "title_bonus_percent": title_bonus_percent,
            "date": self.today(),
            "at": self.now_iso(),
        });
        Ok(self.append_event(event))
    }

    pub fn update_epic(
        &mut self,
        epic_id: &str,
        title: &str,
        description: &str,
        main_dimension: &str,
        title_bonus_dimension: &str,
        title_bonus_percent: f64,
        title_emoji: &str,
    ) -> Result<Value, StoreError> {
        self.ensure_ready();
        let state = self.get_state();
        if !state.epics.contains_key(epic_id) {
            return Err(StoreError::NotFound("里程碑不存在".to_string()));
        }
        validate_epic_fields(title, main_dimension, title_bonus_dimension)?;
        let new_title = title.trim();
        if new_title.is_empty() {
            return Err(StoreError::Invalid("请填写里程碑名称".to_string()));
        }
        let event = json!({
            "type": "EPIC_UPDATED",
            "id": epic_id,
            "name": format!("更新里程碑：{new_title}"),
            "changes": [],
            "title": new_title,
            "description": description.trim(),
            "main_dimension": main_dimension,
            "title_bonus_dimension": title_bonus_dimension,
            "title_bonus_percent": title_bonus_percent,
            "title_emoji": if title_emoji.trim().is_empty() { "🏅" } else { title_emoji.trim() },
            "date": self.today(),
            "at": self.now_iso(),
        });
        Ok(self.append_event(event))
    }

    pub fn complete_epic(&mut self, epic_id: &str, engraving: &str) -> Result<Value, StoreError> {
        self.ensure_ready();
        if engraving.trim().chars().count() < 2 {
            return Err(StoreError::Invalid("请写下 1-2 句结项铭文".to_string()));
        }
        let state = self.get_state();
        let Some(epic) = state.epics.get(epic_id) else {
            return Err(StoreError::NotFound("里程碑不存在".to_string()));
        };
        if epic.status == "completed" {
            return Err(StoreError::Invalid("里程碑已经完成".to_string()));
        }
        let epic_title = epic.title.clone();
        let event = json!({
            "type": "EPIC_COMPLETED",
            "id": epic_id,
            "name": format!("里程碑结项：{epic_title}"),
            "changes": [],
            "engraving": engraving,
            "date": self.today(),
            "at": self.now_iso(),
        });
        Ok(self.append_event(event))
    }

    pub fn create_task(&mut self, params: CreateTask) -> Result<Value, StoreError> {
        self.ensure_ready();
        if params.title.trim().is_empty() {
            return Err(StoreError::Invalid("请填写 Task 标题".to_string()));
        }
        let state = self.get_state();
        if let Some(epic_id) = &params.epic_id {
            if !state.epics.contains_key(epic_id) {
                return Err(StoreError::NotFound("里程碑不存在".to_string()));
            }
        }
        let normalized = normalize_effects(&params.effects)?;
        if normalized.is_empty() {
            return Err(StoreError::Invalid("至少设置一个属性增益或减益".to_string()));
        }
        let event = json!({
            "type": "TASK_CREATED",
            "id": new_event_id(),
            "name": format!("创建 Task：{}", params.title),
            "changes": normalized,
            "title": params.title,
            "epic_id": params.epic_id,
            "effects": normalized,
            "repeatable": params.repeatable,
            "tags": params.tags,
            "date": self.today(),
            "at": self.now_iso(),
        });
        Ok(self.append_event(event))
    }

    pub fn complete_task(&mut self, task_id: &str, note: &str) -> Result<Value, StoreError> {
        self.ensure_ready();
        let state = self.get_state();
        let Some(task) = state.tasks.get(task_id) else {
            return Err(StoreError::NotFound("Task 不存在".to_string()));
        };
        if task.deleted || task.status == "deleted" {
            return Err(StoreError::Invalid("Task 已删除".to_string()));
        }
        if task.status == "completed" {
            return Err(StoreError::Invalid("Task 已经完成".to_string()));
        }
        let inputs: Vec<EffectInput> = task
            .effects
            .iter()
            .map(|effect| EffectInput { dimension: effect.dimension.clone(), delta: effect.delta })
            .collect();
        let effects = apply_title_bonuses(&inputs, &state);
        let san_delta = round1(
            effects
                .iter()
                .filter(|effect| effect.dimension == "san")
                .map(|effect| effect.delta)
                .sum(),
        );
        let current_san = state.dimensions.get("san").and_then(serde_json::Number::as_f64).unwrap_or(0.0);
        if san_delta < 0.0 && current_san + san_delta < 0.0 {
            return Err(StoreError::Invalid(format!(
                "SAN 不足：当前 {}，该 Task 需要 {}",
                fmt_f64(current_san),
                fmt_f64(san_delta.abs())
            )));
        }
        let task_title = task.title.clone();
        let base_effects = task.effects.clone();
        let effects_value = serde_json::to_value(&effects).expect("效果序列化不可失败");
        let event = json!({
            "type": "TASK_COMPLETED",
            "id": task_id,
            "name": format!("记录 Task：{task_title}"),
            "changes": effects_value,
            "note": note,
            "effects": effects_value,
            "base_effects": base_effects,
            "date": self.today(),
            "at": self.now_iso(),
        });
        Ok(self.append_event(event))
    }

    pub fn update_task(&mut self, task_id: &str, params: UpdateTask) -> Result<Value, StoreError> {
        self.ensure_ready();
        let state = self.get_state();
        let Some(task) = state.tasks.get(task_id) else {
            return Err(StoreError::NotFound("Task 不存在".to_string()));
        };
        if task.deleted || task.status == "deleted" {
            return Err(StoreError::Invalid("Task 已删除".to_string()));
        }
        if let Some(epic_id) = &params.epic_id {
            if !state.epics.contains_key(epic_id) {
                return Err(StoreError::NotFound("里程碑不存在".to_string()));
            }
        }
        let Some(raw_effects) = params.effects.as_ref() else {
            // Python：effects=None → 空列表 → 拒绝（更新必须携带效果）。
            return Err(StoreError::Invalid("至少保留一个属性增益或减益".to_string()));
        };
        let normalized = normalize_effects(raw_effects)?;
        if normalized.is_empty() {
            return Err(StoreError::Invalid("至少保留一个属性增益或减益".to_string()));
        }
        let new_title = params
            .title
            .clone()
            .or_else(|| Some(task.title.clone()))
            .unwrap_or_default()
            .trim()
            .to_string();
        if new_title.is_empty() {
            return Err(StoreError::Invalid("请填写 Task 名称".to_string()));
        }
        let repeatable = params.repeatable.unwrap_or(task.repeatable);
        let event = json!({
            "type": "TASK_UPDATED",
            "id": task_id,
            "name": format!("更新 Task：{new_title}"),
            "changes": normalized,
            "title": new_title,
            "epic_id": params.epic_id,
            "effects": normalized,
            "repeatable": repeatable,
            "date": self.today(),
            "at": self.now_iso(),
        });
        Ok(self.append_event(event))
    }

    pub fn delete_task(&mut self, task_id: &str) -> Result<Value, StoreError> {
        self.ensure_ready();
        let state = self.get_state();
        let Some(task) = state.tasks.get(task_id) else {
            return Err(StoreError::NotFound("Task 不存在".to_string()));
        };
        if task.deleted || task.status == "deleted" {
            return Err(StoreError::Invalid("Task 已删除".to_string()));
        }
        let task_title = task.title.clone();
        let deleted_at = self.now_iso();
        let event = json!({
            "type": "TASK_DELETED",
            "id": task_id,
            "name": format!("删除 Task：{task_title}"),
            "changes": [],
            "deleted": true,
            "deleted_at": deleted_at,
            "date": self.today(),
            "at": deleted_at,
        });
        Ok(self.append_event(event))
    }

    pub fn equip_title(&mut self, title_id: &str) -> Result<Value, StoreError> {
        self.ensure_ready();
        let state = self.get_state();
        let Some(title) = state.titles.get(title_id) else {
            return Err(StoreError::Invalid("称号尚未解锁".to_string()));
        };
        if !title.unlocked {
            return Err(StoreError::Invalid("称号尚未解锁".to_string()));
        }
        if state.equipped.iter().any(|id| id == title_id) {
            return Err(StoreError::Invalid("称号已经装备".to_string()));
        }
        if state.equipped.len() >= crate::domain::MAX_EQUIPPED_TITLES {
            return Err(StoreError::Invalid("最多只能装备 3 个称号".to_string()));
        }
        let event = json!({
            "type": "TITLE_EQUIPPED",
            "name": format!("装备称号：{title_id}"),
            "changes": [],
            "title_id": title_id,
            "date": self.today(),
            "at": self.now_iso(),
        });
        Ok(self.append_event(event))
    }

    pub fn unequip_title(&mut self, title_id: &str) -> Result<Value, StoreError> {
        self.ensure_ready();
        let event = json!({
            "type": "TITLE_UNEQUIPPED",
            "name": format!("卸下称号：{title_id}"),
            "changes": [],
            "title_id": title_id,
            "date": self.today(),
            "at": self.now_iso(),
        });
        Ok(self.append_event(event))
    }

    pub fn awaken(&mut self) -> Result<Value, StoreError> {
        self.ensure_ready();
        let state = self.get_state();
        if state.profile.awakened {
            return Err(StoreError::Invalid("已经觉醒".to_string()));
        }
        let event = json!({
            "type": "PROFILE_AWAKENED",
            "name": "提前觉醒",
            "changes": [],
            "date": self.today(),
            "at": self.now_iso(),
        });
        Ok(self.append_event(event))
    }

    pub fn delete_event(&mut self, event_id: &str, note: &str) -> Result<Value, StoreError> {
        self.ensure_ready();
        let events = self.read_events();
        let target = events.iter().find(|event| {
            event.get("event_id").and_then(Value::as_str) == Some(event_id)
                || event.get("id").and_then(Value::as_str) == Some(event_id)
        });
        let Some(target) = target else {
            return Err(StoreError::NotFound("事件不存在".to_string()));
        };
        let target_type = target.get("type").and_then(Value::as_str).unwrap_or("");
        if matches!(
            target_type,
            "SYSTEM_INIT" | "SYSTEM_DAILY_TICK" | "EVENT_DELETED" | "STREAM_CLEARED"
        ) {
            return Err(StoreError::Invalid("系统事件不允许删除".to_string()));
        }
        let target_key = target
            .get("event_id")
            .and_then(Value::as_str)
            .or_else(|| target.get("id").and_then(Value::as_str))
            .unwrap_or_default()
            .to_string();
        let already_deleted = events.iter().any(|event| {
            event.get("type").and_then(Value::as_str) == Some("EVENT_DELETED")
                && event.get("target_event_id").and_then(Value::as_str) == Some(target_key.as_str())
        });
        if already_deleted {
            return Err(StoreError::Invalid("事件已经删除".to_string()));
        }
        let display_name = target
            .get("name")
            .and_then(Value::as_str)
            .filter(|text| !text.is_empty())
            .or_else(|| target.get("title").and_then(Value::as_str))
            .unwrap_or(target_type);
        let deleted_at = self.now_iso();
        let deletion = json!({
            "type": "EVENT_DELETED",
            "name": format!("删除事件：{display_name}"),
            "changes": [],
            "target_event_id": target_key,
            "target_type": target_type,
            "note": note,
            "deleted": true,
            "deleted_at": deleted_at,
            "date": self.today(),
            "at": deleted_at,
        });
        Ok(self.append_event(deletion))
    }
}
