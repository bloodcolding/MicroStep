# ipc-api Specification

## Purpose
TBD - created by archiving change refactor-to-tauri-v2. Update Purpose after archive.
## Requirements
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

### Requirement: 响应信封兼容

每个 command SHALL 返回与原 HTTP 端点结构等价的 JSON 信封：成功时含 `ok: true` 与原有业务字段；失败时含 `ok: false` 与 `error` 信息。错误文案的语义 SHALL 与 HTTP 版一致，使前端渲染层零改动。

#### Scenario: 成功响应形状

- **WHEN** 前端调用任一 command 且业务成功
- **THEN** 返回体的字段结构与其对应 HTTP 端点的历史响应一致

#### Scenario: 业务校验失败

- **WHEN** 提交 SAN 扣减超过当前 SAN 的结算
- **THEN** 返回 `ok: false` 且 `error` 说明拒绝原因，与 HTTP 版语义一致

### Requirement: 前端 API 层透明适配

`frontend/js/api.js` SHALL 保持导出签名 `api(path, options)` 不变，内部将既有 `/api/...` 路径映射为对应 invoke command 并透传错误；其余前端模块 SHALL NOT 感知传输层变化。

#### Scenario: 调用点零改动

- **WHEN** 前端模块以既有签名调用 `api("/api/tasks", { method: "POST", body })`
- **THEN** 请求被路由到 `create_task` command，行为与 HTTP 版一致，调用方代码无需修改

#### Scenario: 错误冒泡

- **WHEN** command 返回 `ok: false`
- **THEN** `api()` 抛出含 `error` 信息的异常，与 HTTP 版非 2xx 时的抛错路径行为一致

