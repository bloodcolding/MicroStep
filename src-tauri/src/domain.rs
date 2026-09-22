//! 领域核心：事件模型 + 纯函数 Reducer（移植自 `backend/domain.py`，行为等价）。
//!
//! 硬约束（golden replay 前提）：重放路径零真实时钟——所有时间只来自事件字段。
//! `initial_state()` 的 `current_day` 是唯一例外（对齐 Python `iso_date()` 锚定当天），
//! 但任何以 SYSTEM_INIT 开头的事件流都会立即覆写该值，故对基准资产不可观测。
//!
//! 与 Python 的已知良性差异（_corpus 内不可观测，如遇分歧以 golden 对照拦截）：
//! - `today()` 用 UTC 日期（std 无时区能力）；本地时区真实时钟随 Tauri 壳批次落位；
//! - 显式 `null` 与键缺省在 Option 类型上合并（Python dict 可区分，corpus 内无此形状，
//!   唯一例外 `unlock_title_id` 用双层 Option 显式建模 tri-state）；
//! - 数值容错（字符串数字、`base_exp` 回退）仅覆盖 corpus 形状。

use std::collections::BTreeMap;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use serde_json::{json, Number, Value};

/// 称号装备槽上限（Python `MAX_EQUIPPED_TITLES`，经 /api/meta 下发）。
pub const MAX_EQUIPPED_TITLES: usize = 3;

/// 六维 pool（不含 san），顺序与 Python `POOL_DIMENSIONS` 一致。
pub const POOL_DIMENSIONS: [&str; 6] = [
    "physical",
    "professional",
    "knowledge",
    "expression",
    "kindness",
    "charm",
];

/// 维度白名单（san + 六维 pool；不存在 willpower/EXP/等级）。
pub fn has_dimension(dimension: &str) -> bool {
    dimension == "san" || POOL_DIMENSIONS.contains(&dimension)
}

pub fn pool_dimensions() -> Vec<&'static str> {
    POOL_DIMENSIONS.to_vec()
}

/// 效果维度白名单顺序（Python `TASK_EFFECT_DIMENSIONS`：san 在前 + 六维）。
pub fn effect_dimensions() -> Vec<&'static str> {
    let mut dims = vec!["san"];
    dims.extend(POOL_DIMENSIONS);
    dims
}

/// 称号加成维度白名单（Python `TITLE_BONUS_DIMENSIONS`：仅六维 pool）。
pub fn title_bonus_dimensions() -> Vec<&'static str> {
    POOL_DIMENSIONS.to_vec()
}

/// 维度元数据（Python `DIMENSION_META` 逐字段等价，/api/meta 下发）。
pub fn dimension_meta() -> Value {
    json!({
        "san": {
            "name": "🧠 SAN",
            "full_name": "理智 / 精力槽",
            "kind": "gauge",
            "description": "每天从 100 开始，按当天事件结算，次日记录最终值后重置。",
            "tags": ["恢复"],
        },
        "physical": {
            "name": "🔋 体质",
            "full_name": "硬件基底",
            "kind": "pool",
            "description": "健康与体能。",
            "tags": ["健康", "体能", "运动"],
        },
        "professional": {
            "name": "⚔️ 专业能力",
            "full_name": "核心资产",
            "kind": "pool",
            "description": "职业、项目与核心能力。",
            "tags": ["工作", "项目", "专业"],
        },
        "knowledge": {
            "name": "📚 知识",
            "full_name": "认知广度",
            "kind": "pool",
            "description": "阅读、学习与认知积累。",
            "tags": ["阅读", "学习", "知识"],
        },
        "expression": {
            "name": "🗣️ 表达",
            "full_name": "输出信噪比",
            "kind": "pool",
            "description": "写作、演讲、交流与语言输出。",
            "tags": ["写作", "表达", "输出"],
        },
        "kindness": {
            "name": "🕊️ 良善",
            "full_name": "正外部性",
            "kind": "pool",
            "description": "助人、贡献与社会责任。",
            "tags": ["助人", "公益", "贡献"],
        },
        "charm": {
            "name": "🌟 魅力",
            "full_name": "外部杠杆",
            "kind": "pool",
            "description": "形象、社交与影响力。",
            "tags": ["社交", "形象", "影响力"],
        },
    })
}

/// 维度显示名（Python `DIMENSION_META[...]["name"]`，称号描述用）。
fn dimension_name(dimension: &str) -> &'static str {
    match dimension {
        "san" => "🧠 SAN",
        "physical" => "🔋 体质",
        "professional" => "⚔️ 专业能力",
        "knowledge" => "📚 知识",
        "expression" => "🗣️ 表达",
        "kindness" => "🕊️ 良善",
        "charm" => "🌟 魅力",
        _ => "⚔️ 专业能力", // Python `.get(dim, DIMENSION_META["professional"])` 回退
    }
}

// ---------------------------------------------------------------------------
// 数值与日期辅助（Python 语义等价）
// ---------------------------------------------------------------------------

/// 等价 Python `round(x, 1)`：`{:.1}` 正确十进制舍入（二进制精确值的就近偶数位）。
pub fn round1(value: f64) -> f64 {
    format!("{value:.1}").parse().unwrap_or(value)
}

/// 等价 Python `round(x, 2)`。
fn round2(value: f64) -> f64 {
    format!("{value:.2}").parse().unwrap_or(value)
}

/// Python `clamp(v, 0, 100)` **位置参数 int 边界**的平局语义：
/// `min(100, x)` 平局（x == 100.0）返回首个参数 int 100；`max(0, m)` 同理 int 0。
/// 故 SAN 触顶/触底时存储 int（JSON "100"/"0"），否则 float。以 Number 保真。
fn py_clamp_number(value: f64) -> Number {
    if value >= 100.0 {
        return Number::from(100i64);
    }
    if value <= 0.0 {
        return Number::from(0i64);
    }
    Number::from_f64(value).expect("有限值")
}

/// Python `round(clamp(float(san), 0, 100), 1)`：int 透传、float 舍入（历史键专用）。
fn py_tick_history_number(san: f64) -> Number {
    if san >= 100.0 {
        return Number::from(100i64);
    }
    if san == 0.0 {
        return Number::from(0i64);
    }
    Number::from_f64(round1(san)).expect("有限值")
}

/// Python `or` 语义：None 与空串均为假值，回退默认。
fn or_default(value: Option<&String>, fallback: String) -> String {
    match value {
        Some(text) if !text.is_empty() => text.clone(),
        _ => fallback,
    }
}

/// Python `float(x)`：数字直取、数字字符串可解析、bool 转 0/1、其余失败。
fn py_float(value: &Value) -> Option<f64> {
    match value {
        Value::Number(number) => number.as_f64(),
        Value::String(text) => text.parse::<f64>().ok(),
        Value::Bool(flag) => Some(if *flag { 1.0 } else { 0.0 }),
        _ => None,
    }
}

/// Python `%g` 形状（Rust f64 Display 输出无尾零最短形：15.0 → "15"）。
fn py_g(value: f64) -> String {
    format!("{value}")
}

/// UTC 今日（YYYY-MM-DD）。仅空流场景可观测（见模块注释）。
fn today() -> String {
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64;
    civil_from_days(secs.div_euclid(86400))
}

/// Howard Hinnant civil 算法：自纪元天数 → YYYY-MM-DD。
fn civil_from_days(days: i64) -> String {
    let z = days + 719468;
    let era = if z >= 0 { z } else { z - 146096 } / 146097;
    let doe = z - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    format!("{y:04}-{m:02}-{d:02}")
}

/// YYYY-MM-DD → 自纪元天数（非法输入返回 None）。
pub fn days_from_date(text: &str) -> Option<i64> {
    let mut parts = text.split('-');
    let year = parts.next()?.parse::<i64>().ok()?;
    let month = parts.next()?.parse::<i64>().ok()?;
    let day = parts.next()?.parse::<i64>().ok()?;
    if !(1..=12).contains(&month) || !(1..=31).contains(&day) || parts.next().is_some() {
        return None;
    }
    let year = if month <= 2 { year - 1 } else { year };
    let era = if year >= 0 { year } else { year - 399 } / 400;
    let yoe = year - era * 400;
    let mp = if month > 2 { month - 3 } else { month + 9 };
    let doy = (153 * mp + 2) / 5 + day - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    Some(era * 146097 + doe - 719468)
}

pub fn date_from_days(days: i64) -> String {
    civil_from_days(days)
}

// ---------------------------------------------------------------------------
// 事件 schema
// ---------------------------------------------------------------------------

/// 属性变化（Python `Effect`）。加成结算后额外含 base_delta / bonus_percent；
/// 未加成时这两键**缺省**（非 null），对应 Python dict 无此键。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Effect {
    pub dimension: String,
    pub delta: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub base_delta: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bonus_percent: Option<f64>,
}

impl Effect {
    fn plain(dimension: &str, delta: f64) -> Effect {
        Effect { dimension: dimension.to_string(), delta, base_delta: None, bonus_percent: None }
    }
}

/// 存储命令/测试构造用的原始效果输入（dimension + delta）。
#[derive(Debug, Clone, PartialEq)]
pub struct EffectInput {
    pub dimension: String,
    pub delta: f64,
}

/// 事件公共字段（event_id/created_at 由 append_event 统一注入，故可缺省）。
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct EventCommon {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(default)]
    pub changes: Vec<Effect>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub date: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub at: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub event_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub created_at: Option<String>,
}

/// flatten 缓冲下 serde 的 Option 会把显式 null 折叠为缺省；
/// 直接走 `Value::deserialize` 保留 tri-state（缺省→default None，null→Some(Null)）。
fn deserialize_explicit_value<'de, D>(deserializer: D) -> Result<Option<Value>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    Value::deserialize(deserializer).map(Some)
}

/// 14 种事件（serde tag 对应 Python TypedDict Literal 标签，schema 字节级不变）。
/// `Unknown` 承载未知类型事件：重放结算忽略之，但保留在事件流投影中。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Event {
    SystemInit {
        #[serde(flatten)]
        common: EventCommon,
    },
    SystemDailyTick {
        #[serde(flatten)]
        common: EventCommon,
    },
    ProfileAwakened {
        #[serde(flatten)]
        common: EventCommon,
    },
    TaskCreated {
        id: String,
        #[serde(default)]
        title: Option<String>,
        #[serde(default)]
        epic_id: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        effects: Option<Vec<Effect>>,
        #[serde(default)]
        repeatable: bool,
        #[serde(default)]
        tags: Vec<String>,
        // 旧版单属性事件回退字段（无 effects 键时经 effects 回退路径生效）。
        #[serde(rename = "dimension", default, skip_serializing_if = "Option::is_none")]
        legacy_dimension: Option<String>,
        #[serde(rename = "exp_delta", default, skip_serializing_if = "Option::is_none")]
        legacy_exp_delta: Option<f64>,
        #[serde(rename = "san_delta", default, skip_serializing_if = "Option::is_none")]
        legacy_san_delta: Option<f64>,
        #[serde(flatten)]
        common: EventCommon,
    },
    TaskCompleted {
        id: String,
        #[serde(default)]
        note: Option<String>,
        #[serde(default)]
        effects: Option<Vec<Effect>>,
        #[serde(default)]
        base_effects: Option<Vec<Effect>>,
        #[serde(flatten)]
        common: EventCommon,
    },
    TaskUpdated {
        id: String,
        #[serde(default)]
        title: Option<String>,
        #[serde(default)]
        epic_id: Option<String>,
        #[serde(default)]
        effects: Option<Vec<Effect>>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        repeatable: Option<bool>,
        #[serde(flatten)]
        common: EventCommon,
    },
    TaskDeleted {
        id: String,
        #[serde(default)]
        deleted: bool,
        #[serde(default)]
        deleted_at: Option<String>,
        #[serde(flatten)]
        common: EventCommon,
    },
    EpicCreated {
        id: String,
        #[serde(default)]
        title: Option<String>,
        #[serde(default)]
        description: Option<String>,
        #[serde(default)]
        main_dimension: Option<String>,
        #[serde(default)]
        title_emoji: Option<String>,
        #[serde(default)]
        title_bonus_dimension: Option<String>,
        // Python float(x or 0) 容错语义要求数值以 Value 承载（字符串数字/假值回退 0）。
        #[serde(default, skip_serializing_if = "Option::is_none")]
        title_bonus_percent: Option<Value>,
        // tri-state：None=键缺省（序列化跳过）、Null=显式 null、字符串原样。
        #[serde(
            default,
            skip_serializing_if = "Option::is_none",
            deserialize_with = "deserialize_explicit_value"
        )]
        unlock_title_id: Option<Value>,
        #[serde(flatten)]
        common: EventCommon,
    },
    EpicUpdated {
        id: String,
        #[serde(default)]
        title: Option<String>,
        #[serde(default)]
        description: Option<String>,
        #[serde(default)]
        main_dimension: Option<String>,
        #[serde(default)]
        title_emoji: Option<String>,
        #[serde(default)]
        title_bonus_dimension: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        title_bonus_percent: Option<Value>,
        #[serde(flatten)]
        common: EventCommon,
    },
    EpicCompleted {
        id: String,
        #[serde(default)]
        engraving: Option<String>,
        #[serde(flatten)]
        common: EventCommon,
    },
    TitleEquipped {
        title_id: String,
        #[serde(flatten)]
        common: EventCommon,
    },
    TitleUnequipped {
        title_id: String,
        #[serde(flatten)]
        common: EventCommon,
    },
    EventDeleted {
        target_event_id: String,
        #[serde(default)]
        target_type: Option<String>,
        #[serde(default)]
        note: Option<String>,
        #[serde(default)]
        deleted: bool,
        #[serde(default)]
        deleted_at: Option<String>,
        #[serde(flatten)]
        common: EventCommon,
    },
    StreamCleared {
        #[serde(flatten)]
        common: EventCommon,
    },
    /// 未知事件类型（serde 反序列化失败的兜底；不参与结算，保留在事件流投影）。
    Unknown {
        #[serde(skip)]
        unknown_tag: Option<String>,
        #[serde(flatten)]
        value: Value,
    },
}

impl Event {
    /// 从 JSON Value 解析事件：已知 tag 走强类型，未知/畸形 tag 落入 `Unknown`
    /// （对齐 Python「损坏行跳过 + 未知类型忽略」容错语义，design D6）。
    pub fn from_value(value: Value) -> Event {
        let unknown_tag = value.get("type").and_then(Value::as_str).map(String::from);
        match serde_json::from_value::<Event>(value) {
            Ok(event) => event,
            Err(_) => Event::Unknown { unknown_tag, value: Value::Null },
        }
    }

    /// 事件 type 标签（含 Unknown 的原始 tag）。
    pub fn type_tag(&self) -> &str {
        match self {
            Event::SystemInit { .. } => "SYSTEM_INIT",
            Event::SystemDailyTick { .. } => "SYSTEM_DAILY_TICK",
            Event::ProfileAwakened { .. } => "PROFILE_AWAKENED",
            Event::TaskCreated { .. } => "TASK_CREATED",
            Event::TaskCompleted { .. } => "TASK_COMPLETED",
            Event::TaskUpdated { .. } => "TASK_UPDATED",
            Event::TaskDeleted { .. } => "TASK_DELETED",
            Event::EpicCreated { .. } => "EPIC_CREATED",
            Event::EpicUpdated { .. } => "EPIC_UPDATED",
            Event::EpicCompleted { .. } => "EPIC_COMPLETED",
            Event::TitleEquipped { .. } => "TITLE_EQUIPPED",
            Event::TitleUnequipped { .. } => "TITLE_UNEQUIPPED",
            Event::EventDeleted { .. } => "EVENT_DELETED",
            Event::StreamCleared { .. } => "STREAM_CLEARED",
            Event::Unknown { unknown_tag, .. } => unknown_tag.as_deref().unwrap_or(""),
        }
    }
}

// ---------------------------------------------------------------------------
// 聚合状态
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Profile {
    pub name: String,
    pub created_at: Option<String>,
    pub awakened: bool,
    pub action_count: i64,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Meta {
    pub current_day: Option<String>,
    pub last_tick_day: Option<String>,
    /// 值以 Number 承载：SAN 触顶/触底时为 int（Python clamp 平局语义），其余 float。
    pub daily_san_history: BTreeMap<String, Number>,
}

/// Task 投影（基于事件流派生；删除/更新不回写历史事件）。
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Task {
    pub id: String,
    pub event_id: Option<String>,
    pub title: String,
    pub epic_id: Option<String>,
    pub effects: Vec<Effect>,
    pub status: String,
    pub deleted: bool,
    pub deleted_at: Option<String>,
    pub updated_at: Option<String>,
    pub created_at: Option<String>,
    pub completed_at: Option<String>,
    pub progress: i64,
    pub repeatable: bool,
    pub times_completed: i64,
    pub last_completed_at: Option<String>,
    pub tags: Vec<String>,
}

/// 里程碑（与称号一一配对）。
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Epic {
    pub id: String,
    pub title: String,
    pub description: String,
    pub main_dimension: String,
    pub title_id: String,
    pub title_emoji: String,
    pub title_bonus_dimension: String,
    pub title_bonus_percent: f64,
    pub status: String,
    pub task_ids: Vec<String>,
    pub created_at: Option<String>,
    pub completed_at: Option<String>,
    pub engraving: String,
}

/// 称号。
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Title {
    pub id: String,
    pub name: String,
    pub emoji: String,
    pub description: String,
    pub milestone_id: String,
    pub target_dimension: String,
    pub bonus_percent: f64,
    pub unlocked: bool,
}

/// 派生 SAN 概览（build_state 阶段计算）。
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct DerivedState {
    pub san: f64,
    pub san_start: f64,
    pub san_change: f64,
    pub total_actions: i64,
    pub pool_dimensions: Vec<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct DailySan {
    pub day: Option<String>,
    pub start: f64,
    pub current: f64,
    pub change: f64,
    pub history: BTreeMap<String, Number>,
}

/// 事件流投影条目（build_event_stream 产物；含 tombstone 标记）。
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct EventStreamItem {
    pub event_id: Option<String>,
    #[serde(rename = "type")]
    pub event_type: String,
    pub name: String,
    pub changes: Vec<Effect>,
    pub created_at: Option<String>,
    pub deleted: bool,
    pub deleted_at: Option<String>,
    pub deletion_event_id: Option<String>,
    pub details: StreamDetails,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct StreamDetails {
    pub text: Option<String>,
    pub title: Option<String>,
    pub tags: Vec<String>,
    pub dimension: Option<String>,
    pub note: Option<String>,
    pub engraving: Option<String>,
    pub repeatable: bool,
}

/// 聚合状态（重放产物）。logs 条目为 task/epic 两种形状，以 Value 承载；
/// dimensions 值以 Number 承载（SAN 触顶/触底时为 int，见 `py_clamp_number`）。
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct State {
    pub profile: Profile,
    pub dimensions: BTreeMap<String, Number>,
    pub meta: Meta,
    pub epics: BTreeMap<String, Epic>,
    pub tasks: BTreeMap<String, Task>,
    pub titles: BTreeMap<String, Title>,
    pub equipped: Vec<String>,
    pub logs: Vec<Value>,
    pub derived: DerivedState,
    pub daily_san: DailySan,
    pub event_stream: Vec<EventStreamItem>,
}

/// 初始状态。注意：`meta.current_day` 锚定真实时钟（Python `iso_date()`），
/// 仅空流场景可见；带 SYSTEM_INIT 的流会被覆写为事件 date。
pub fn initial_state() -> State {
    let mut dimensions = BTreeMap::new();
    dimensions.insert("san".to_string(), Number::from_f64(100.0).expect("有限值"));
    for key in POOL_DIMENSIONS {
        dimensions.insert(key.to_string(), Number::from_f64(0.0).expect("有限值"));
    }
    State {
        profile: Profile {
            name: "成长者".to_string(),
            created_at: None,
            awakened: false,
            action_count: 0,
        },
        dimensions,
        meta: Meta {
            current_day: Some(today()),
            last_tick_day: None,
            daily_san_history: BTreeMap::new(),
        },
        ..Default::default()
    }
}

// ---------------------------------------------------------------------------
// Reducer
// ---------------------------------------------------------------------------

/// 纯函数 Reducer：按事件类型分发（等价 Python `apply_event`，无副作用）。
pub fn apply_event(state: &mut State, event: &Event) {
    match event {
        Event::SystemInit { common } => {
            let created = or_default(common.date.as_ref(), today());
            state.profile.created_at = Some(created.clone());
            state.meta.current_day = Some(created);
        }
        Event::SystemDailyTick { common } => apply_daily_tick(state, common),
        Event::ProfileAwakened { .. } => state.profile.awakened = true,
        Event::TaskCreated { .. } => apply_task_created(state, event),
        Event::TaskCompleted { .. } => apply_task_completed(state, event),
        Event::TaskUpdated { .. } => apply_task_updated(state, event),
        Event::TaskDeleted { .. } => apply_task_deleted(state, event),
        Event::EpicCreated { .. } => apply_epic_created(state, event),
        Event::EpicUpdated { .. } => apply_epic_updated(state, event),
        Event::EpicCompleted { .. } => apply_epic_completed(state, event),
        Event::TitleEquipped { title_id, .. } => {
            let can_equip = state
                .titles
                .get(title_id)
                .is_some_and(|title| title.unlocked)
                && !state.equipped.contains(title_id);
            if can_equip && state.equipped.len() < MAX_EQUIPPED_TITLES {
                state.equipped.push(title_id.clone());
            }
        }
        Event::TitleUnequipped { title_id, .. } => {
            state.equipped.retain(|id| id != title_id);
        }
        // 控制面事件与未知事件不参与结算（投影层另行处理）。
        Event::EventDeleted { .. } | Event::StreamCleared { .. } | Event::Unknown { .. } => {}
    }
}

fn apply_daily_tick(state: &mut State, common: &EventCommon) {
    let new_day = or_default(common.date.as_ref(), today());
    if let Some(previous_day) = state.meta.current_day.clone() {
        if previous_day != new_day {
            let san = state.dimensions.get("san").and_then(Number::as_f64).unwrap_or(100.0);
            state
                .meta
                .daily_san_history
                .insert(previous_day, py_tick_history_number(san));
        }
    }
    state.meta.current_day = Some(new_day.clone());
    state.meta.last_tick_day = Some(new_day);
    state
        .dimensions
        .insert("san".to_string(), Number::from_f64(100.0).expect("有限值"));
}

/// Python `_effects_from_event`：原始 effects 规范化（白名单过滤 + round1 + 去零）。
fn normalized_effects(raw: Option<&Vec<Effect>>) -> Vec<Effect> {
    let mut effects = Vec::new();
    if let Some(items) = raw {
        for item in items {
            if !has_dimension(&item.dimension) {
                continue;
            }
            let delta = round1(item.delta);
            if delta != 0.0 {
                effects.push(Effect::plain(&item.dimension, delta));
            }
        }
    }
    effects
}

/// Python `_effects_from_event` 的旧版单属性回退分支（dimension/exp_delta/san_delta）。
fn legacy_effects(
    dimension: &Option<String>,
    exp_delta: &Option<f64>,
    san_delta: &Option<f64>,
) -> Vec<Effect> {
    let Some(dimension) = dimension else {
        return Vec::new();
    };
    if !has_dimension(dimension) {
        return Vec::new();
    }
    let exp = round1(exp_delta.unwrap_or(0.0));
    let san = round1(san_delta.unwrap_or(0.0));
    let mut effects = Vec::new();
    if dimension != "san" && exp != 0.0 {
        effects.push(Effect::plain(dimension, exp));
    }
    if san != 0.0 {
        effects.push(Effect::plain("san", san));
    }
    effects
}

fn apply_task_created(state: &mut State, event: &Event) {
    let Event::TaskCreated {
        id,
        title,
        epic_id,
        effects,
        repeatable,
        tags,
        legacy_dimension,
        legacy_exp_delta,
        legacy_san_delta,
        common,
    } = event
    else {
        unreachable!("分发器保证变体正确");
    };
    let task_id = id.clone();
    let mut task_effects = normalized_effects(effects.as_ref());
    if task_effects.is_empty() {
        task_effects = legacy_effects(legacy_dimension, legacy_exp_delta, legacy_san_delta);
    }
    let task = Task {
        id: task_id.clone(),
        event_id: common.event_id.clone(),
        title: title.clone().unwrap_or_else(|| "未命名 Task".to_string()),
        epic_id: epic_id.clone(),
        effects: task_effects,
        status: "active".to_string(),
        deleted: false,
        deleted_at: None,
        updated_at: None,
        created_at: Some(or_default(common.date.as_ref(), today())),
        completed_at: None,
        progress: 0,
        repeatable: *repeatable,
        times_completed: 0,
        last_completed_at: None,
        tags: tags.clone(),
    };
    state.tasks.insert(task_id.clone(), task);
    if let Some(epic_id) = epic_id {
        if let Some(epic) = state.epics.get_mut(epic_id) {
            if !epic.task_ids.contains(&task_id) {
                epic.task_ids.push(task_id);
            }
        }
    }
}

fn apply_task_completed(state: &mut State, event: &Event) {
    let Event::TaskCompleted { id, effects: event_effects, common, .. } = event else {
        unreachable!("分发器保证变体正确");
    };
    let Some(existing) = state.tasks.get(id) else { return };
    if existing.deleted {
        return;
    }
    let task_title = existing.title.clone();
    let task_effects = existing.effects.clone();
    let task_repeatable = existing.repeatable;
    let task_times = existing.times_completed;
    let action_day = or_default(common.date.as_ref(), today());

    let mut effects = normalized_effects(event_effects.as_ref());
    if effects.is_empty() {
        effects = task_effects
            .iter()
            .map(|item| Effect::plain(&item.dimension, round1(item.delta)))
            .collect();
    }

    for effect in &effects {
        if effect.dimension == "san" {
            let current = state.dimensions.get("san").and_then(Number::as_f64).unwrap_or(100.0);
            // Python：clamp(float(cur) + delta, 0, 100)——位置参数 int 边界平局产 int。
            state.dimensions.insert("san".to_string(), py_clamp_number(current + effect.delta));
        } else if let Some(current) = state.dimensions.get_mut(&effect.dimension) {
            let settled = round1((current.as_f64().unwrap_or(0.0) + effect.delta).max(0.0));
            *current = Number::from_f64(settled).expect("有限值");
        }
    }

    let times_completed = task_times + 1;
    let task = state.tasks.get_mut(id).unwrap();
    task.times_completed = times_completed;
    task.last_completed_at = Some(action_day.clone());
    task.progress = if !task_repeatable { 100 } else { (times_completed * 10).min(100) };
    if task_repeatable {
        task.status = "active".to_string();
    } else {
        task.status = "completed".to_string();
        task.completed_at = Some(action_day.clone());
    }

    state.logs.push(json!({
        "type": "task",
        "id": id,
        "event_id": common.event_id,
        "title": task_title,
        "effects": effects,
        "repeatable": task_repeatable,
        "times_completed": times_completed,
        "date": action_day,
        "at": common.at,
    }));
}

fn apply_task_updated(state: &mut State, event: &Event) {
    let Event::TaskUpdated {
        id,
        title,
        epic_id,
        effects,
        repeatable,
        common,
    } = event
    else {
        unreachable!("分发器保证变体正确");
    };
    let Some(existing) = state.tasks.get(id) else { return };
    if existing.deleted {
        return;
    }
    let old_epic_id = existing.epic_id.clone();
    let old_title = existing.title.clone();
    let old_status = existing.status.clone();
    let old_times = existing.times_completed;
    let old_repeatable = existing.repeatable;

    let new_title = match title {
        Some(text) if !text.is_empty() => text.clone(),
        _ => old_title,
    };
    // Python「键存在即赋值」：store 生成的更新事件恒携带 epic_id（含显式 null=移出）。
    let new_epic_id = epic_id.clone();
    let (new_repeatable, revived) = match repeatable {
        Some(flag) => (*flag, *flag && old_status == "completed"),
        None => (old_repeatable, false),
    };
    let new_effects = normalized_effects(effects.as_ref());

    let updated_at = common
        .created_at
        .clone()
        .or_else(|| common.at.clone())
        .or_else(|| common.date.clone())
        .map(|value| or_default(Some(&value), today()))
        .unwrap_or_else(today);

    let task = state.tasks.get_mut(id).unwrap();
    task.title = new_title;
    task.epic_id = new_epic_id.clone();
    if revived {
        task.status = "active".to_string();
        task.completed_at = None;
        task.progress = (old_times * 10).min(100);
    }
    task.repeatable = new_repeatable;
    if !new_effects.is_empty() {
        task.effects = new_effects;
    }
    task.updated_at = Some(updated_at);
    sync_task_epic(state, id, old_epic_id, new_epic_id);
}

fn apply_task_deleted(state: &mut State, event: &Event) {
    let Event::TaskDeleted { id, deleted_at, common, .. } = event else {
        unreachable!("分发器保证变体正确");
    };
    let Some(existing) = state.tasks.get(id) else { return };
    if existing.deleted {
        return;
    }
    let epic_id = existing.epic_id.clone();
    let stamp = deleted_at
        .clone()
        .or_else(|| common.created_at.clone())
        .or_else(|| common.at.clone())
        .or_else(|| common.date.clone())
        .map(|value| or_default(Some(&value), today()))
        .unwrap_or_else(today);
    let task = state.tasks.get_mut(id).unwrap();
    task.deleted = true;
    task.deleted_at = Some(stamp);
    task.status = "deleted".to_string();
    if let Some(epic_id) = epic_id {
        if let Some(epic) = state.epics.get_mut(&epic_id) {
            epic.task_ids.retain(|task_id| task_id != id);
        }
    }
}

fn sync_task_epic(
    state: &mut State,
    task_id: &str,
    old_epic_id: Option<String>,
    new_epic_id: Option<String>,
) {
    if let Some(old_id) = &old_epic_id {
        if Some(old_id) != new_epic_id.as_ref() {
            if let Some(epic) = state.epics.get_mut(old_id) {
                epic.task_ids.retain(|id| id != task_id);
            }
        }
    }
    if let Some(new_id) = &new_epic_id {
        if let Some(epic) = state.epics.get_mut(new_id) {
            if !epic.task_ids.contains(&task_id.to_string()) {
                epic.task_ids.push(task_id.to_string());
            }
        }
    }
}

/// 称号加成维度白名单（仅六维 pool，san 不可；Python `TITLE_BONUS_DIMENSIONS`）。
fn is_bonus_dimension(dimension: &str) -> bool {
    POOL_DIMENSIONS.contains(&dimension)
}

/// Python `event.get("title_bonus_percent") or 0` + `float()` 容错 → round2。
fn coerce_bonus_percent(value: &Option<Value>) -> f64 {
    match value {
        None => 0.0,
        Some(inner) => round2(py_float(inner).unwrap_or(0.0)),
    }
}

fn title_description(title: &str, dimension: &str, bonus_percent: f64) -> String {
    format!(
        "完成里程碑「{title}」解锁；{}变化 {}%",
        dimension_name(dimension),
        py_g(bonus_percent)
    )
}

fn apply_epic_created(state: &mut State, event: &Event) {
    let Event::EpicCreated {
        id,
        title,
        description,
        main_dimension,
        title_emoji,
        title_bonus_dimension,
        title_bonus_percent,
        common,
        ..
    } = event
    else {
        unreachable!("分发器保证变体正确");
    };
    let epic_id = id.clone();
    let epic_title = title.clone().unwrap_or_else(|| "未命名里程碑".to_string());
    let bonus_dimension = match title_bonus_dimension.as_deref() {
        Some(dimension) if !dimension.is_empty() && is_bonus_dimension(dimension) => {
            dimension.to_string()
        }
        _ => "professional".to_string(),
    };
    let bonus_percent = coerce_bonus_percent(title_bonus_percent);
    let emoji = title_emoji.clone().unwrap_or_else(|| "🏅".to_string());

    state.epics.insert(
        epic_id.clone(),
        Epic {
            id: epic_id.clone(),
            title: epic_title.clone(),
            description: description.clone().unwrap_or_default(),
            main_dimension: main_dimension.clone().unwrap_or_else(|| "professional".to_string()),
            title_id: epic_id.clone(),
            title_emoji: emoji.clone(),
            title_bonus_dimension: bonus_dimension.clone(),
            title_bonus_percent: bonus_percent,
            status: "active".to_string(),
            task_ids: Vec::new(),
            created_at: Some(or_default(common.date.as_ref(), today())),
            completed_at: None,
            engraving: String::new(),
        },
    );
    state.titles.insert(
        epic_id.clone(),
        Title {
            id: epic_id.clone(),
            name: epic_title.clone(),
            emoji,
            description: title_description(&epic_title, &bonus_dimension, bonus_percent),
            milestone_id: epic_id.clone(),
            target_dimension: bonus_dimension,
            bonus_percent,
            unlocked: false,
        },
    );
}

fn apply_epic_updated(state: &mut State, event: &Event) {
    let Event::EpicUpdated {
        id,
        title,
        description,
        main_dimension,
        title_emoji,
        title_bonus_dimension,
        title_bonus_percent,
        ..
    } = event
    else {
        unreachable!("分发器保证变体正确");
    };
    let Some(epic) = state.epics.get(id) else { return };
    let old_title = epic.title.clone();
    let old_description = epic.description.clone();
    let old_main = epic.main_dimension.clone();
    let old_emoji = epic.title_emoji.clone();
    let old_bonus_dim = epic.title_bonus_dimension.clone();
    let old_bonus_percent = epic.title_bonus_percent;

    let new_title = match title {
        Some(text) if !text.is_empty() => text.clone(),
        _ => old_title.clone(),
    };
    let new_main = or_default(main_dimension.as_ref(), old_main);
    let bonus_dimension = match title_bonus_dimension.as_deref() {
        Some(dimension) if !dimension.is_empty() && is_bonus_dimension(dimension) => {
            dimension.to_string()
        }
        Some(_) => "professional".to_string(),
        None => old_bonus_dim.clone(),
    };
    let bonus_percent = match title_bonus_percent {
        // 键缺省 → 沿用现值（Python get(key, epic.get(...))）；显式 null → 0。
        None => old_bonus_percent,
        Some(value) => round2(py_float(value).unwrap_or(0.0)),
    };
    let new_emoji = or_default(title_emoji.as_ref(), old_emoji.clone());
    let new_description = description.clone().unwrap_or(old_description.clone());

    let epic = state.epics.get_mut(id).unwrap();
    epic.title = new_title.clone();
    epic.description = new_description;
    epic.main_dimension = new_main;
    epic.title_emoji = new_emoji.clone();
    epic.title_bonus_dimension = bonus_dimension.clone();
    epic.title_bonus_percent = bonus_percent;
    let final_title = epic.title.clone();
    if let Some(title_entry) = state.titles.get_mut(id) {
        title_entry.name = new_title;
        title_entry.emoji = new_emoji;
        title_entry.description =
            title_description(&final_title, &bonus_dimension, bonus_percent);
        title_entry.target_dimension = bonus_dimension;
        title_entry.bonus_percent = bonus_percent;
    }
}

fn apply_epic_completed(state: &mut State, event: &Event) {
    let Event::EpicCompleted { id, engraving, common } = event else {
        unreachable!("分发器保证变体正确");
    };
    let Some(epic) = state.epics.get(id) else { return };
    let epic_title = epic.title.clone();
    let completed_day = or_default(common.date.as_ref(), today());
    let engraving_text = engraving.clone().unwrap_or_default();

    let epic = state.epics.get_mut(id).unwrap();
    epic.status = "completed".to_string();
    epic.completed_at = Some(completed_day.clone());
    epic.engraving = engraving_text.clone();
    if let Some(title) = state.titles.get_mut(id) {
        title.unlocked = true;
    }
    state.logs.push(json!({
        "type": "epic",
        "id": id,
        "event_id": common.event_id,
        "title": epic_title,
        "engraving": engraving_text,
        "date": completed_day,
        "at": common.at,
    }));
}

// ---------------------------------------------------------------------------
// 状态与事件流
// ---------------------------------------------------------------------------

/// 重放事件流得到聚合状态（等价 Python `build_state`：跳过控制面事件结算、
/// 追加 derived/daily_san/event_stream 投影）。
pub fn build_state(events: &[Event]) -> State {
    let mut state = initial_state();
    for event in events {
        match event {
            Event::EventDeleted { .. } | Event::StreamCleared { .. } | Event::Unknown { .. } => {
                continue;
            }
            _ => apply_event(&mut state, event),
        }
    }
    let san = state.dimensions.get("san").and_then(Number::as_f64).unwrap_or(100.0);
    state.derived = DerivedState {
        san: round1(san),
        san_start: 100.0,
        san_change: round1(san - 100.0),
        total_actions: state.profile.action_count,
        pool_dimensions: POOL_DIMENSIONS.iter().map(|key| key.to_string()).collect(),
    };
    state.daily_san = DailySan {
        day: Some(state.meta.current_day.clone().unwrap_or_else(today)),
        start: 100.0,
        current: round1(san),
        change: round1(san - 100.0),
        history: state.meta.daily_san_history.clone(),
    };
    state.event_stream = build_event_stream(events);
    // Python：round(v, 1) if isinstance(v, float) else v——int 透传，float 舍入。
    for value in state.dimensions.values_mut() {
        if let Some(number) = value.as_f64() {
            *value = Number::from_f64(round1(number)).expect("有限值");
        }
    }
    state
}

/// 称号百分比加成结算（等价 Python `apply_title_bonuses`：
/// 白名单过滤、零增益丢弃、负增益同样放大、多称号可加叠加）。
pub fn apply_title_bonuses(effects: &[EffectInput], state: &State) -> Vec<Effect> {
    let mut bonuses: BTreeMap<String, f64> = BTreeMap::new();
    for title_id in &state.equipped {
        let Some(title) = state.titles.get(title_id) else { continue };
        if !title.unlocked {
            continue;
        }
        if !is_bonus_dimension(&title.target_dimension) {
            continue;
        }
        let dimension = title.target_dimension.clone();
        let total = bonuses.get(&dimension).copied().unwrap_or(0.0) + title.bonus_percent;
        bonuses.insert(dimension, total);
    }

    let mut result = Vec::new();
    for effect in effects {
        if !has_dimension(&effect.dimension) {
            continue;
        }
        let base_delta = round1(effect.delta);
        if base_delta == 0.0 {
            continue;
        }
        let bonus_percent = bonuses.get(&effect.dimension).copied().unwrap_or(0.0);
        let final_delta = round1(base_delta * (1.0 + bonus_percent / 100.0));
        result.push(Effect {
            dimension: effect.dimension.clone(),
            delta: final_delta,
            base_delta: Some(base_delta),
            bonus_percent: Some(bonus_percent),
        });
    }
    result
}

/// Python `_normalize_change_list`：changes 规范化（白名单 + round1 + 去零 + 去加成键）。
fn normalize_change_list(changes: &[Effect]) -> Vec<Effect> {
    let mut result = Vec::new();
    for item in changes {
        if !has_dimension(&item.dimension) {
            continue;
        }
        let delta = round1(item.delta);
        if delta != 0.0 {
            result.push(Effect::plain(&item.dimension, delta));
        }
    }
    result
}

/// Python `_event_name` 的映射回退（无显式 name 时）。
fn fallback_event_name(event: &Event, title: Option<&String>) -> String {
    let title_text = title.map(String::as_str).unwrap_or("");
    match event {
        Event::TaskCreated { .. } => format!("创建 Task：{title_text}"),
        Event::TaskCompleted { .. } => format!("记录 Task：{title_text}"),
        Event::TaskUpdated { .. } => format!("更新 Task：{title_text}"),
        Event::TaskDeleted { .. } => format!("删除 Task：{title_text}"),
        Event::EpicCreated { .. } => format!("创建里程碑：{title_text}"),
        Event::EpicUpdated { .. } => format!("更新里程碑：{title_text}"),
        Event::EpicCompleted { .. } => format!("里程碑结项：{title_text}"),
        Event::ProfileAwakened { .. } => "提前觉醒".to_string(),
        Event::TitleEquipped { title_id, .. } => format!("装备称号：{title_id}"),
        Event::TitleUnequipped { title_id, .. } => format!("卸下称号：{title_id}"),
        Event::SystemDailyTick { .. } => "每日结算".to_string(),
        Event::SystemInit { .. } => "系统初始化".to_string(),
        Event::EventDeleted { .. } | Event::StreamCleared { .. } => String::new(),
        Event::Unknown { unknown_tag, .. } => unknown_tag.clone().unwrap_or_default(),
    }
}

/// 事件流投影（等价 Python `build_event_stream`：tombstone 标记 + 控制面事件剔除）。
fn build_event_stream(events: &[Event]) -> Vec<EventStreamItem> {
    // target_event_id → (deleted_at, deletion_event_id)
    let mut deletions: BTreeMap<String, (Option<String>, Option<String>)> = BTreeMap::new();
    for event in events {
        if let Event::EventDeleted { target_event_id, deleted_at, common, .. } = event {
            if target_event_id.is_empty() {
                continue;
            }
            let stamp = deleted_at
                .clone()
                .or_else(|| common.created_at.clone())
                .or_else(|| common.at.clone())
                .or_else(|| common.date.clone());
            deletions.insert(target_event_id.clone(), (stamp, common.event_id.clone()));
        }
    }

    let mut stream = Vec::new();
    for event in events {
        if matches!(event, Event::EventDeleted { .. } | Event::StreamCleared { .. }) {
            continue;
        }

        // (variant_id, title, note, tags, dimension, engraving, repeatable, effects)
        let projection = match event {
            Event::TaskCreated {
                id,
                title,
                effects,
                repeatable,
                tags,
                legacy_dimension,
                common,
                ..
            } => (
                Some(id.clone()),
                title.clone(),
                None,
                tags.clone(),
                legacy_dimension.clone(),
                None,
                *repeatable,
                normalized_effects(effects.as_ref()),
                common,
            ),
            Event::TaskCompleted { id, note, effects, common, .. } => (
                Some(id.clone()),
                None,
                note.clone(),
                Vec::new(),
                None,
                None,
                false,
                normalized_effects(effects.as_ref()),
                common,
            ),
            Event::TaskUpdated { id, title, effects, repeatable, common, .. } => (
                Some(id.clone()),
                title.clone(),
                None,
                Vec::new(),
                None,
                None,
                repeatable.unwrap_or(false),
                normalized_effects(effects.as_ref()),
                common,
            ),
            Event::TaskDeleted { id, common, .. } => {
                (Some(id.clone()), None, None, Vec::new(), None, None, false, Vec::new(), common)
            }
            Event::EpicCreated { id, title, common, .. } => (
                Some(id.clone()),
                title.clone(),
                None,
                Vec::new(),
                None,
                None,
                false,
                Vec::new(),
                common,
            ),
            Event::EpicUpdated { id, title, common, .. } => (
                Some(id.clone()),
                title.clone(),
                None,
                Vec::new(),
                None,
                None,
                false,
                Vec::new(),
                common,
            ),
            Event::EpicCompleted { id, engraving, common } => (
                Some(id.clone()),
                None,
                None,
                Vec::new(),
                None,
                engraving.clone(),
                false,
                Vec::new(),
                common,
            ),
            Event::TitleEquipped { common, .. }
            | Event::TitleUnequipped { common, .. }
            | Event::SystemInit { common }
            | Event::SystemDailyTick { common }
            | Event::ProfileAwakened { common } => {
                (None, None, None, Vec::new(), None, None, false, Vec::new(), common)
            }
            Event::Unknown { value, .. } => {
                // 未知事件原样投影：name 缺省回退 type 字符串（Python mapping.get(type, type)）。
                let name = value
                    .get("name")
                    .and_then(Value::as_str)
                    .map(String::from)
                    .unwrap_or_else(|| fallback_event_name(event, None));
                stream.push(EventStreamItem {
                    event_id: ["event_id", "id"]
                        .iter()
                        .find_map(|key| value.get(*key).and_then(Value::as_str))
                        .map(String::from),
                    event_type: event.type_tag().to_string(),
                    name,
                    changes: Vec::new(),
                    created_at: ["created_at", "at", "date"]
                        .iter()
                        .find_map(|key| value.get(*key).and_then(Value::as_str))
                        .map(String::from),
                    deleted: false,
                    deleted_at: None,
                    deletion_event_id: None,
                    details: StreamDetails {
                        text: None,
                        title: value.get("title").and_then(Value::as_str).map(String::from),
                        tags: Vec::new(),
                        dimension: None,
                        note: None,
                        engraving: None,
                        repeatable: false,
                    },
                });
                continue;
            }
            Event::EventDeleted { .. } | Event::StreamCleared { .. } => unreachable!(),
        };
        let (variant_id, title, note, tags, dimension, engraving, repeatable, effects, common) =
            projection;

        let resolved_id = common.event_id.clone().or(variant_id);
        let deletion = resolved_id.as_ref().and_then(|id| deletions.get(id));
        let mut changes = normalize_change_list(&common.changes);
        if changes.is_empty() {
            changes = fallback_changes(event, effects);
        }
        let name = common
            .name
            .clone()
            .unwrap_or_else(|| fallback_event_name(event, title.as_ref()));
        stream.push(EventStreamItem {
            event_id: resolved_id,
            event_type: event.type_tag().to_string(),
            name,
            changes,
            created_at: common
                .created_at
                .clone()
                .or_else(|| common.at.clone())
                .or_else(|| common.date.clone()),
            deleted: deletion.is_some(),
            deleted_at: deletion.and_then(|(stamp, _)| stamp.clone()),
            deletion_event_id: deletion.and_then(|(_, event_id)| event_id.clone()),
            details: StreamDetails {
                text: None,
                title,
                tags,
                dimension,
                note,
                engraving,
                repeatable,
            },
        });
    }
    stream
}

/// changes 为空时回退 effects/legacy（Python `_event_changes` 的第二分支）。
fn fallback_changes(event: &Event, effects: Vec<Effect>) -> Vec<Effect> {
    if !effects.is_empty() {
        return effects;
    }
    match event {
        Event::TaskCreated { legacy_dimension, legacy_exp_delta, legacy_san_delta, .. } => {
            legacy_effects(legacy_dimension, legacy_exp_delta, legacy_san_delta)
        }
        _ => Vec::new(),
    }
}

/// 归一化序列化：递归键排序、紧凑分隔符、非 ASCII 不转义
/// （等价 Python `json.dumps(state, sort_keys=True, ensure_ascii=False,
/// separators=(",", ":"))`，golden 对照的唯一口径）。
pub fn canonical_state_json(state: &State) -> String {
    // 经 Value 中转使所有 Map 走 BTreeMap 键排序；serde_json 序列化非 ASCII 不转义。
    serde_json::to_value(state)
        .map(|value| value.to_string())
        .expect("State 序列化不可失败")
}
