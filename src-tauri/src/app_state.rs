//! IPC 信封层：AppState 托管 EventStore（Mutex），提供 14 个 command 的
//! 参数强制转换与响应信封，严格镜像 `backend/server.py` 各 handler 语义
//! （ipc-api spec「响应信封兼容」），使前端渲染层零改动。
//!
//! 复刻的 Python 怪癖（均以 server.py 现行为唯一真源）：
//! - `str(body.get(k, ""))`：显式 null → `"None"`；
//! - `str(x or 默认)`：假值（null/false/0/""/[]/{}）回退默认后再 str；
//! - `float(x or 0)`：字符串可解析（含首尾空白），失败文案复刻 ValueError/TypeError；
//! - `isinstance(effects, list)`：非列表视为缺省（None）；
//! - KeyError 的 `str(exc)` 是参数 repr → `error` 带 **单引号**；
//! - `bool(x)`（Python 真值）：字符串 `"false"` 为真。

use std::path::PathBuf;
use std::sync::{Mutex, MutexGuard};

use serde_json::{json, Value};

use crate::domain::{
    dimension_meta, effect_dimensions, has_dimension, pool_dimensions, title_bonus_dimensions,
    EffectInput, MAX_EQUIPPED_TITLES,
};
use crate::store::{fmt_f64, CreateTask, EventStore, StoreError, UpdateTask};
use crate::sync;

/// Python 分发层的两类可预期异常：ValueError → 400 信封原文；
/// TypeError 等 → 500 信封加「服务器内部错误: 」前缀。
enum CoerceError {
    Invalid(String),
    Internal(String),
}

/// 共享状态：单一 EventStore 由 Mutex 串行化（等价 Python RLock）。
pub struct AppState {
    store: Mutex<EventStore>,
    /// 数据目录（sync.json / .git 所在地，由事件流路径推导）。
    data_dir: PathBuf,
}

impl AppState {
    /// 以已就绪的 EventStore 构造（AppData 路径解析见 main 接线 / data_dir）。
    pub fn new(store: EventStore) -> Self {
        let data_dir = store
            .path()
            .parent()
            .map(std::path::Path::to_path_buf)
            .unwrap_or_default();
        Self { store: Mutex::new(store), data_dir }
    }

    /// Python RLock 异常后仍可用；等价地忽略锁毒化继续持有。
    fn lock(&self) -> MutexGuard<'_, EventStore> {
        self.store.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    /// 启动补结算（Python `main()`：ensure_initialized + ensure_daily_ticks）。
    pub fn ensure_bootstrapped(&self) {
        let mut store = self.lock();
        store.ensure_initialized();
        store.ensure_daily_ticks();
    }

    /// Ticker 单轮检查入口（Python `Ticker._run` 调用的就是它）。
    pub fn ensure_daily_ticks(&self) {
        self.lock().ensure_daily_ticks();
    }

    // ------------------------------------------------------------------
    // 同步 command（data-sync spec：3 个信封方法，错误都在信封内）
    // ------------------------------------------------------------------

    /// 由数据目录 + 事件流路径构造同步引擎。
    fn sync_engine(&self) -> sync::SyncEngine {
        let events_path = self.lock().path().to_path_buf();
        sync::SyncEngine::new(self.data_dir.clone(), events_path)
    }

    /// `sync_get_config`：读同步配置（PAT 脱敏回显，仅末 4 位可辨识）。
    pub fn sync_get_config(&self) -> Value {
        let cfg = sync::load_sync_config(&self.data_dir);
        json!({
            "ok": true,
            "remote_url": cfg.remote_url,
            "pat": sync::mask_pat(&cfg.pat),
            "branch": cfg.branch,
            "last_sync_at": cfg.last_sync_at,
            "last_result": cfg.last_result,
        })
    }

    /// `sync_set_config`：部分更新语义（未携带字段不变；pat 空串清除）。
    pub fn sync_set_config(
        &self,
        remote_url: Option<String>,
        pat: Option<String>,
        branch: Option<String>,
    ) -> Value {
        let mut cfg = sync::load_sync_config(&self.data_dir);
        sync::apply_sync_config_update(
            &mut cfg,
            &sync::SyncConfigUpdate { remote_url, pat, branch },
        );
        match sync::save_sync_config(&self.data_dir, &cfg) {
            Ok(()) => json!({ "ok": true }),
            Err(err) => json!({ "ok": false, "error": err.to_string() }),
        }
    }

    /// `sync_now`：完整 pull-merge-push，成功信封含 pulled/pushed/merged 统计。
    pub fn sync_now(&self) -> Value {
        match self.sync_engine().sync_now(&self.store) {
            Ok(outcome) => json!({
                "ok": true,
                "pulled": outcome.pulled,
                "pushed": outcome.pushed,
                "merged": outcome.merged,
            }),
            Err(err) => json!({ "ok": false, "error": err.message() }),
        }
    }

    /// 启动 best-effort pull（lib.rs setup spawn 调用；失败仅记 last_result）。
    pub fn startup_pull(&self) {
        let _ = self.sync_engine().startup_pull(&self.store);
    }

    // ------------------------------------------------------------------
    // 读命令（HTTP GET 语义：200 + 原始 payload，无 ok 字段）
    // ------------------------------------------------------------------

    /// `GET /api/state`：重放事件流返回当前状态。
    pub fn get_state(&self) -> Value {
        let state = self.lock().get_state();
        serde_json::to_value(&state).expect("State 序列化不可失败")
    }

    /// `GET /api/meta`：维度与称号定义（不触碰事件流）。
    pub fn get_meta(&self) -> Value {
        json!({
            "dimensions": dimension_meta(),
            "pool_dimensions": pool_dimensions(),
            "effect_dimensions": effect_dimensions(),
            "title_bonus_dimensions": title_bonus_dimensions(),
            "max_equipped_titles": MAX_EQUIPPED_TITLES,
        })
    }

    // ------------------------------------------------------------------
    // 写命令（HTTP POST 语义：{ok, event, state} / {ok, state} 信封）
    // ------------------------------------------------------------------

    /// `POST /api/epics` → create_epic。
    pub fn create_epic(&self, body: &Value) -> Value {
        let percent = match float_or_zero(body, "title_bonus_percent") {
            Ok(value) => value,
            Err(err) => return coerce_error_envelope(err),
        };
        let mut store = self.lock();
        match store.create_epic(
            &str_field(body, "title"),
            &str_field(body, "description").trim(),
            &str_or_field(body, "main_dimension", "professional"),
            &str_or_field(body, "title_bonus_dimension", "professional"),
            percent,
            Some(&str_or_field(body, "title_emoji", "🏅")),
        ) {
            Ok(event) => ok_event_envelope(&event, &mut store),
            Err(err) => store_error_envelope(err),
        }
    }

    /// `POST /api/epics/{id}/update` → update_epic（strip 由 store 层负责）。
    pub fn update_epic(&self, epic_id: &str, body: &Value) -> Value {
        let percent = match float_or_zero(body, "title_bonus_percent") {
            Ok(value) => value,
            Err(err) => return coerce_error_envelope(err),
        };
        let mut store = self.lock();
        match store.update_epic(
            epic_id,
            &str_field(body, "title"),
            &str_field(body, "description"),
            &str_or_field(body, "main_dimension", "professional"),
            &str_or_field(body, "title_bonus_dimension", "professional"),
            percent,
            &str_or_field(body, "title_emoji", "🏅"),
        ) {
            Ok(event) => ok_event_envelope(&event, &mut store),
            Err(err) => store_error_envelope(err),
        }
    }

    /// `POST /api/epics/{id}/complete` → complete_epic。
    pub fn complete_epic(&self, epic_id: &str, body: &Value) -> Value {
        let engraving = str_field(body, "engraving");
        let mut store = self.lock();
        match store.complete_epic(epic_id, &engraving) {
            Ok(event) => ok_event_envelope(&event, &mut store),
            Err(err) => store_error_envelope(err),
        }
    }

    /// `POST /api/tasks` → create_task。
    pub fn create_task(&self, body: &Value) -> Value {
        let effects = match effects_field(body, "effects") {
            Ok(value) => value,
            Err(err) => return store_error_envelope(err),
        };
        let mut store = self.lock();
        match store.create_task(CreateTask {
            title: str_field(body, "title"),
            epic_id: clean_id(body, "epic_id"),
            effects: effects.unwrap_or_default(),
            repeatable: bool_field(body, "repeatable"),
            tags: tags_field(body),
        }) {
            Ok(event) => ok_event_envelope(&event, &mut store),
            Err(err) => store_error_envelope(err),
        }
    }

    /// `POST /api/tasks/{id}/log`（/complete 别名共用）→ log_task。
    pub fn log_task(&self, task_id: &str, body: &Value) -> Value {
        let note = str_field(body, "note");
        let mut store = self.lock();
        match store.complete_task(task_id, &note) {
            Ok(event) => ok_event_envelope(&event, &mut store),
            Err(err) => store_error_envelope(err),
        }
    }

    /// `POST /api/tasks/{id}/update` → update_task。
    pub fn update_task(&self, task_id: &str, body: &Value) -> Value {
        let effects = match effects_field(body, "effects") {
            Ok(value) => value,
            Err(err) => return store_error_envelope(err),
        };
        let mut store = self.lock();
        match store.update_task(
            task_id,
            UpdateTask {
                title: non_empty(&str_field(body, "title").trim()),
                epic_id: clean_id(body, "epic_id"),
                effects,
                repeatable: optional_bool_field(body, "repeatable"),
            },
        ) {
            Ok(event) => ok_event_envelope(&event, &mut store),
            Err(err) => store_error_envelope(err),
        }
    }

    /// `POST /api/tasks/{id}/delete` → delete_task（body 忽略，对齐 HTTP 版）。
    pub fn delete_task(&self, task_id: &str) -> Value {
        let mut store = self.lock();
        match store.delete_task(task_id) {
            Ok(event) => ok_event_envelope(&event, &mut store),
            Err(err) => store_error_envelope(err),
        }
    }

    /// `POST /api/events/{id}/delete` → delete_event。
    pub fn delete_event(&self, event_id: &str, body: &Value) -> Value {
        let note = str_field(body, "note");
        let mut store = self.lock();
        match store.delete_event(event_id, &note) {
            Ok(event) => ok_event_envelope(&event, &mut store),
            Err(err) => store_error_envelope(err),
        }
    }

    /// `POST /api/titles/equip` → equip_title。
    pub fn equip_title(&self, body: &Value) -> Value {
        let title_id = clean_id(body, "title_id").unwrap_or_default();
        let mut store = self.lock();
        match store.equip_title(&title_id) {
            Ok(event) => ok_event_envelope(&event, &mut store),
            Err(err) => store_error_envelope(err),
        }
    }

    /// `POST /api/titles/unequip` → unequip_title。
    pub fn unequip_title(&self, body: &Value) -> Value {
        let title_id = clean_id(body, "title_id").unwrap_or_default();
        let mut store = self.lock();
        match store.unequip_title(&title_id) {
            Ok(event) => ok_event_envelope(&event, &mut store),
            Err(err) => store_error_envelope(err),
        }
    }

    /// `POST /api/awaken` → awaken。
    pub fn awaken(&self) -> Value {
        let mut store = self.lock();
        match store.awaken() {
            Ok(event) => ok_event_envelope(&event, &mut store),
            Err(err) => store_error_envelope(err),
        }
    }

    /// `POST /api/system/tick` → system_tick：Python `manual_tick()` 只做
    /// ensure_ready（返回的 tick 字典被丢弃、不落盘），信封为 {ok, state}。
    pub fn system_tick(&self) -> Value {
        let mut store = self.lock();
        store.ensure_initialized();
        store.ensure_daily_ticks();
        let state = store.get_state();
        json!({ "ok": true, "state": state })
    }
}

// ---------------------------------------------------------------------------
// 信封构造
// ---------------------------------------------------------------------------

/// server.py `_event_response`：{ok, event, state}（state 为追加后重放）。
fn ok_event_envelope(event: &Value, store: &mut EventStore) -> Value {
    json!({ "ok": true, "event": event, "state": store.get_state() })
}

/// server.py 分发层：ValueError → `str(exc)` 原文；KeyError → `str(exc)`
/// 是参数的 repr，即**带单引号**（如 `"'Task 不存在'"`）。
fn store_error_envelope(err: StoreError) -> Value {
    match err {
        StoreError::Invalid(msg) => json!({ "ok": false, "error": msg }),
        StoreError::NotFound(msg) => json!({ "ok": false, "error": format!("'{msg}'") }),
    }
}

/// 强制转换失败的信封：ValueError 原文；TypeError → 「服务器内部错误: 」前缀。
fn coerce_error_envelope(err: CoerceError) -> Value {
    match err {
        CoerceError::Invalid(msg) => json!({ "ok": false, "error": msg }),
        CoerceError::Internal(msg) => json!({ "ok": false, "error": format!("服务器内部错误: {msg}") }),
    }
}

// ---------------------------------------------------------------------------
// Python 强制转换语义（str / or / bool / float / isinstance）
// ---------------------------------------------------------------------------

/// Python 真值表：None/false/0（含 -0.0、0.0）/""/[]/{} 为假（NaN 为真）。
fn py_truthy(value: &Value) -> bool {
    match value {
        Value::Null => false,
        Value::Bool(flag) => *flag,
        Value::Number(number) => number.as_f64().map(|v| v != 0.0).unwrap_or(true),
        Value::String(text) => !text.is_empty(),
        Value::Array(items) => !items.is_empty(),
        Value::Object(fields) => !fields.is_empty(),
    }
}

/// Python `str(x)`：Null→"None"、bool→"True"/"False"、整数无尾零、浮点带 .0。
/// 容器走 repr 形状（键序为序列化序，Python 为插入序——corpus 外良性差异）。
fn py_str(value: &Value) -> String {
    match value {
        Value::Null => "None".to_string(),
        Value::Bool(flag) => (if *flag { "True" } else { "False" }).to_string(),
        Value::String(text) => text.clone(),
        Value::Number(number) => number_str(number),
        Value::Array(items) => {
            let parts: Vec<String> = items.iter().map(py_repr).collect();
            format!("[{}]", parts.join(", "))
        }
        Value::Object(fields) => {
            let parts: Vec<String> = fields
                .iter()
                .map(|(key, value)| format!("'{key}': {}", py_repr(value)))
                .collect();
            format!("{{{}}}", parts.join(", "))
        }
    }
}

/// Python repr：字符串加单引号，其余沿用 str 形状。
fn py_repr(value: &Value) -> String {
    match value {
        Value::String(text) => format!("'{text}'"),
        other => py_str(other),
    }
}

/// JSON 数字 → Python str：i64/u64 直取，f64 复用 store::fmt_f64（100.0 → "100.0"）。
fn number_str(number: &serde_json::Number) -> String {
    if let Some(i) = number.as_i64() {
        return i.to_string();
    }
    if let Some(u) = number.as_u64() {
        return u.to_string();
    }
    fmt_f64(number.as_f64().unwrap_or(f64::NAN))
}

/// `str(body.get(key, ""))`：缺省空串；显式 null → "None"。
fn str_field(body: &Value, key: &str) -> String {
    match body.get(key) {
        Some(value) => py_str(value),
        None => String::new(),
    }
}

/// `str(body.get(key) or default)`：假值回退默认。
fn str_or_field(body: &Value, key: &str, default: &str) -> String {
    match body.get(key) {
        Some(value) if py_truthy(value) => py_str(value),
        _ => default.to_string(),
    }
}

/// `bool(body.get(key))`（缺省 null → false）。
fn bool_field(body: &Value, key: &str) -> bool {
    body.get(key).map(py_truthy).unwrap_or(false)
}

/// `body.get(key) if "repeatable" in body else None`：键存在才取 Python 真值。
fn optional_bool_field(body: &Value, key: &str) -> Option<bool> {
    body.as_object()?.get(key).map(py_truthy)
}

/// server.py `_clean_id`：null/缺省 → None；str(x).strip()，空串 → None。
fn clean_id(body: &Value, key: &str) -> Option<String> {
    match body.get(key) {
        Some(Value::Null) | None => None,
        Some(value) => non_empty(py_str(value).trim()),
    }
}

/// 空串归一为 None（`x or None` 语义）。
fn non_empty(text: &str) -> Option<String> {
    if text.is_empty() {
        None
    } else {
        Some(text.to_string())
    }
}

/// `float(body.get(key) or 0)`：假值回退 0；字符串可解析（含首尾空白；
/// "inf"/"nan"/数字下划线字面量等 Python 特例 corpus 外，不复刻）。
fn float_or_zero(body: &Value, key: &str) -> Result<f64, CoerceError> {
    match body.get(key) {
        Some(value) if py_truthy(value) => py_float_value(value),
        _ => Ok(0.0),
    }
}

/// Python `float(x)`：数字直取、字符串解析、bool → 0/1；
/// ValueError → Invalid（原文），TypeError → Internal（原文，信封层加前缀）。
fn py_float_value(value: &Value) -> Result<f64, CoerceError> {
    match value {
        Value::Number(number) => number.as_f64().ok_or_else(|| {
            CoerceError::Internal(
                "float() argument must be a string or a real number, not 'object'".to_string(),
            )
        }),
        Value::String(text) => text.trim().parse::<f64>().map_err(|_| {
            CoerceError::Invalid(format!("could not convert string to float: '{text}'"))
        }),
        Value::Bool(flag) => Ok(if *flag { 1.0 } else { 0.0 }),
        Value::Array(_) => Err(CoerceError::Internal(
            "float() argument must be a string or a real number, not 'list'".to_string(),
        )),
        Value::Object(_) => Err(CoerceError::Internal(
            "float() argument must be a string or a real number, not 'dict'".to_string(),
        )),
        Value::Null => Err(CoerceError::Internal(
            "float() argument must be a string or a real number, not 'NoneType'".to_string(),
        )),
    }
}

/// `effects=body.get("effects") if isinstance(x, list) else None`：
/// 非列表（含缺省/null）→ None；逐项强转 EffectInput。
/// 校验顺序复刻 Python `_normalize_effects`：dimension 白名单先于 delta 数值。
fn effects_field(body: &Value, key: &str) -> Result<Option<Vec<EffectInput>>, StoreError> {
    let Some(Value::Array(items)) = body.get(key) else {
        return Ok(None); // isinstance(effects, list) 不成立 → None
    };
    let mut effects = Vec::new();
    for item in items {
        let Some(fields) = item.as_object() else {
            continue; // Python：非 dict 项 continue 跳过
        };
        let dimension = fields.get("dimension").map(py_str).unwrap_or_default();
        if !has_dimension(&dimension) {
            return Err(StoreError::Invalid("Task 属性不合法".to_string()));
        }
        let delta = match fields.get("delta") {
            None => 0.0, // 缺省 0（round 后为 0，store 层跳过）
            Some(value) => match py_float_value(value) {
                Ok(value) => value,
                Err(_) => return Err(StoreError::Invalid("Task 数值不合法".to_string())),
            },
        };
        effects.push(EffectInput { dimension, delta });
    }
    Ok(Some(effects))
}

/// `tags=body.get("tags") if isinstance(x, list) else []`；
/// 非字符串项以 py_str 入库（Python 保留原始值——前端只发字符串，corpus 外良性差异）。
fn tags_field(body: &Value) -> Vec<String> {
    match body.get("tags") {
        Some(Value::Array(items)) => items.iter().map(py_str).collect(),
        _ => Vec::new(),
    }
}
