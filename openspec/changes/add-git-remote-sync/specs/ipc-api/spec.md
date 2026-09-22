# ipc-api · 规格增量

## MODIFIED Requirements

### Requirement: IPC command 契约

系统 SHALL 提供恰好 17 个 Tauri IPC command：既有 14 个业务 command（`get_state`、`get_meta`、`create_epic`、`update_epic`、`complete_epic`、`create_task`、`log_task`、`update_task`、`delete_task`、`equip_title`、`unequip_title`、`awaken`、`system_tick`、`delete_event`，契约与参数校验规则保持不变，其中 `log_task` SHALL 兼容旧 `POST /api/tasks/{id}/complete` 的语义别名）+ 3 个同步 command：`sync_get_config`（读同步配置，PAT 脱敏回显）、`sync_set_config`（部分更新语义写配置）、`sync_now`（触发完整同步，成功信封含 `pulled`/`pushed`/`merged` 统计）。同步 command 参数键 SHALL 遵循既有 camelCase 约定（如 `remoteUrl`）。

#### Scenario: 命令集完备

- **WHEN** 审计 command 注册表
- **THEN** 存在上列 17 个 command 且无多余业务 command，既有 14 个的参数与对应 HTTP 端点请求体字段一致

#### Scenario: 旧接口别名兼容

- **WHEN** 前端以旧 complete 语义调用 `log_task`
- **THEN** 行为与 HTTP 版 `POST /api/tasks/{id}/complete`（等价于记录一次 Task）一致

#### Scenario: 同步命令信封

- **WHEN** 调用 `sync_now` 且同步成功
- **THEN** 返回 `ok: true` 与 `pulled`/`pushed`/`merged` 统计；失败时 `ok: false` 且 `error` 遵循 data-sync 错误分类
