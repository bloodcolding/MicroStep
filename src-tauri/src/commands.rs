//! 17 个 `#[tauri::command]` 薄封装（ipc-api spec 命令白名单，恰 17 个无多余）：
//! 参数逐字段对应 HTTP 请求体并以 `Option<Value>` 区分「字段缺省」与「显式 null」，
//! 重组为 body 后交给 AppState 信封方法（全部强制转换语义收敛在那里）。
//! 异步命令跑在 tokio 线程池（Python ThreadingHTTPServer 的等价并发形态）；
//! Tauri 约束：async + 借用参数须返回 Result（Err 分支永不使用，错误都在信封内）。

use std::sync::Arc;

use serde_json::{Map, Value};
use tauri::State;

use crate::app_state::AppState;

type SharedState<'a> = State<'a, Arc<AppState>>;

/// 重组请求体：None = 字段缺省（不写入），Some(Null) = 显式 null（保留）。
fn arg_body(entries: &[(&str, Option<Value>)]) -> Value {
    let mut body = Map::new();
    for (key, value) in entries {
        if let Some(value) = value {
            body.insert((*key).to_string(), value.clone());
        }
    }
    Value::Object(body)
}

#[tauri::command]
pub async fn get_state(state: SharedState<'_>) -> Result<Value, String> {
    Ok(state.get_state())
}

#[tauri::command]
pub async fn get_meta(state: SharedState<'_>) -> Result<Value, String> {
    Ok(state.get_meta())
}

#[tauri::command]
pub async fn create_epic(
    state: SharedState<'_>,
    title: Option<Value>,
    description: Option<Value>,
    main_dimension: Option<Value>,
    title_bonus_dimension: Option<Value>,
    title_bonus_percent: Option<Value>,
    title_emoji: Option<Value>,
) -> Result<Value, String> {
    let body = arg_body(&[
        ("title", title),
        ("description", description),
        ("main_dimension", main_dimension),
        ("title_bonus_dimension", title_bonus_dimension),
        ("title_bonus_percent", title_bonus_percent),
        ("title_emoji", title_emoji),
    ]);
    Ok(state.create_epic(&body))
}

#[tauri::command]
pub async fn update_epic(
    state: SharedState<'_>,
    epic_id: String,
    title: Option<Value>,
    description: Option<Value>,
    main_dimension: Option<Value>,
    title_bonus_dimension: Option<Value>,
    title_bonus_percent: Option<Value>,
    title_emoji: Option<Value>,
) -> Result<Value, String> {
    let body = arg_body(&[
        ("title", title),
        ("description", description),
        ("main_dimension", main_dimension),
        ("title_bonus_dimension", title_bonus_dimension),
        ("title_bonus_percent", title_bonus_percent),
        ("title_emoji", title_emoji),
    ]);
    Ok(state.update_epic(&epic_id, &body))
}

#[tauri::command]
pub async fn complete_epic(
    state: SharedState<'_>,
    epic_id: String,
    engraving: Option<Value>,
) -> Result<Value, String> {
    let body = arg_body(&[("engraving", engraving)]);
    Ok(state.complete_epic(&epic_id, &body))
}

#[tauri::command]
pub async fn create_task(
    state: SharedState<'_>,
    title: Option<Value>,
    epic_id: Option<Value>,
    effects: Option<Value>,
    repeatable: Option<Value>,
    tags: Option<Value>,
) -> Result<Value, String> {
    let body = arg_body(&[
        ("title", title),
        ("epic_id", epic_id),
        ("effects", effects),
        ("repeatable", repeatable),
        ("tags", tags),
    ]);
    Ok(state.create_task(&body))
}

#[tauri::command]
pub async fn log_task(
    state: SharedState<'_>,
    task_id: String,
    note: Option<Value>,
) -> Result<Value, String> {
    let body = arg_body(&[("note", note)]);
    Ok(state.log_task(&task_id, &body))
}

#[tauri::command]
pub async fn update_task(
    state: SharedState<'_>,
    task_id: String,
    title: Option<Value>,
    epic_id: Option<Value>,
    effects: Option<Value>,
    repeatable: Option<Value>,
) -> Result<Value, String> {
    let body = arg_body(&[
        ("title", title),
        ("epic_id", epic_id),
        ("effects", effects),
        ("repeatable", repeatable),
    ]);
    Ok(state.update_task(&task_id, &body))
}

#[tauri::command]
pub async fn delete_task(state: SharedState<'_>, task_id: String) -> Result<Value, String> {
    Ok(state.delete_task(&task_id))
}

#[tauri::command]
pub async fn delete_event(
    state: SharedState<'_>,
    event_id: String,
    note: Option<Value>,
) -> Result<Value, String> {
    let body = arg_body(&[("note", note)]);
    Ok(state.delete_event(&event_id, &body))
}

#[tauri::command]
pub async fn equip_title(state: SharedState<'_>, title_id: Option<Value>) -> Result<Value, String> {
    let body = arg_body(&[("title_id", title_id)]);
    Ok(state.equip_title(&body))
}

#[tauri::command]
pub async fn unequip_title(state: SharedState<'_>, title_id: Option<Value>) -> Result<Value, String> {
    let body = arg_body(&[("title_id", title_id)]);
    Ok(state.unequip_title(&body))
}

#[tauri::command]
pub async fn awaken(state: SharedState<'_>) -> Result<Value, String> {
    Ok(state.awaken())
}

#[tauri::command]
pub async fn system_tick(state: SharedState<'_>) -> Result<Value, String> {
    Ok(state.system_tick())
}

// 同步 command（data-sync spec）：参数键经 Tauri v2 映射为 camelCase（remoteUrl）。

/// `sync_get_config`：读同步配置（PAT 脱敏回显）。
#[tauri::command]
pub async fn sync_get_config(state: SharedState<'_>) -> Result<Value, String> {
    Ok(state.sync_get_config())
}

/// `sync_set_config`：部分更新语义写配置（前端参数键 camelCase：remoteUrl/pat/branch）。
#[tauri::command]
pub async fn sync_set_config(
    state: SharedState<'_>,
    remote_url: Option<String>,
    pat: Option<String>,
    branch: Option<String>,
) -> Result<Value, String> {
    Ok(state.sync_set_config(remote_url, pat, branch))
}

/// `sync_now`：触发完整同步（fetch → union merge → 快照 commit → push）。
#[tauri::command]
pub async fn sync_now(state: SharedState<'_>) -> Result<Value, String> {
    Ok(state.sync_now())
}
