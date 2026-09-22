# data-sync · 规格增量

## ADDED Requirements

### Requirement: 同步配置持久化

同步配置 SHALL 持久化于数据目录内 `sync.json`（与 `events.jsonl` 同目录），字段：`remote_url`（HTTPS 远端地址）、`pat`（Personal Access Token，明文存储）、`branch`（同步分支，默认 `main`）、`last_sync_at` 与 `last_result`（最近一次同步结果，供界面展示）。未配置 `remote_url` 时同步能力 SHALL 视为未启用，应用其余功能不受影响。`sync_get_config` 回显时 PAT SHALL 脱敏（仅保留末 4 位可辨识）。

#### Scenario: 保存并脱敏回显

- **WHEN** 用户通过 `sync_set_config` 保存 remote_url 与 PAT 后调用 `sync_get_config`
- **THEN** 配置落盘 `sync.json`，回显含 remote_url / branch / last_sync_at / last_result，PAT 仅显示末 4 位

#### Scenario: 未配置时同步未启用

- **WHEN** `sync.json` 不存在或 `remote_url` 为空
- **THEN** 启动 pull 跳过、`sync_now` 返回未配置错误，其余功能与现状一致

#### Scenario: 部分更新语义

- **WHEN** `sync_set_config` 仅携带部分字段
- **THEN** 未携带字段保持原值不变；`pat` 显式传空字符串时清除既有 PAT

### Requirement: Union Merge 语义

同步合并 SHALL 在应用层按事件行执行 union merge，SHALL NOT 使用 git 自带的 merge 机器。算法：解析双方全部行提取 `event_id`；本地全部行按原序保留；远端独有行（id 不在本地 id 集）按远端相对顺序追加到尾部；同 id 且解析后语义相等的行视为同一事件去重（保留本地行）；同 id 但解析后内容不同的行 SHALL 触发整次同步拒绝（事件不可变红线），双端文件保持原样，错误信息点名冲突 `event_id`。无法解析的行：本地无 id 行原样保留，远端无 id 行仅当字节级不存在于本地时追加。合并 SHALL 确定（相同输入产生字节级相同输出）。

#### Scenario: 远端独有事件并入

- **WHEN** 远端存在本地没有的 3 个事件且无冲突
- **THEN** 合并文件 = 本地全部行 + 3 个远端行按远端序追加，统计 pulled=3

#### Scenario: 同 id 语义相等去重

- **WHEN** 同一 `event_id` 双端均存在且解析后 JSON 相等（允许键序/空白差异）
- **THEN** 保留本地行，不重复追加

#### Scenario: 同 id 内容冲突拒绝

- **WHEN** 同一 `event_id` 双端内容解析后不同
- **THEN** 同步中止且双端文件字节不变，错误信息含冲突 `event_id`

#### Scenario: 合并确定性

- **WHEN** 以相同（本地, 远端）输入重复执行合并
- **THEN** 输出文件字节级一致

### Requirement: 同步流程

一次完整同步（`sync_now`）SHALL 依序执行：fetch 远端分支 → 读远端 tip 的 `events.jsonl` blob（无需工作区 checkout）→ union merge → 若本地或合并结果相对上次 commit 有变化则写 blob/tree/commit（merge 产生合流时 commit 以本地 tip 与远端 tip 双 parent 记录）→ push 至远端分支 → 更新 `last_sync_at`/`last_result`。push 遇 non-fast-forward（远端在 fetch 后又被推进）SHALL 重跑完整 fetch+merge 循环，重试上限 2 次；仍失败 SHALL 返回并发冲突错误，本地已合并结果保留、数据无损。commit 仅在同步时写入（快照式历史），事件追加本身 SHALL NOT 自动 commit。网络操作 SHALL 有超时（默认 30 秒）。

#### Scenario: 双向同步

- **WHEN** 本地与远端各有对方没有的事件且无冲突
- **THEN** 合并后双端文件一致，返回 {pulled, pushed, merged} 统计

#### Scenario: push 并发冲突重试

- **WHEN** push 时远端已被另一设备推进
- **THEN** 重跑 fetch+merge（≤2 次），成功则完成同步，仍冲突则报"远端并发更新"且本地数据无损

#### Scenario: 快照式 commit

- **WHEN** 普通业务操作追加事件
- **THEN** 不产生 git commit；仅同步时写入快照 commit

### Requirement: Bootstrap 场景

首次同步 SHALL 覆盖三种起点：远端为空仓库（无 commits）→ 本地当前 `events.jsonl` 直接 commit 并推送（存量数据目录 .git 无 commits 时同此路径）；本地无事件而远端有 → 远端全部事件按序落盘（纯恢复）；双端均有数据 → 标准 union merge。

#### Scenario: 空远端推种子

- **WHEN** 远端仓库无任何 commit 且本地已有事件流
- **THEN** 本地快照 commit 成为首个 commit 并推送成功

#### Scenario: 纯恢复

- **WHEN** 本地 `events.jsonl` 为空/不存在而远端有完整历史
- **THEN** 远端事件按远端序全量落盘，统计 pulled=远端事件数

### Requirement: 同步触发

同步 SHALL 有两个触发点：应用启动时异步执行 best-effort pull（fetch → merge → 本地落盘；不 push），失败仅记 `last_result` 与日志，SHALL NOT 阻塞启动、SHALL NOT 弹窗；用户在同步设置面板手动触发 `sync_now`（完整 pull-merge-push），结果完整回传前端展示。

#### Scenario: 启动 pull 失败不阻断

- **WHEN** 启动时远端不可达
- **THEN** 应用正常进入可用状态，`last_result` 记录失败原因，无弹窗

#### Scenario: 手动同步结果展示

- **WHEN** 用户点击「同步」
- **THEN** 前端展示本次 pulled/pushed/merged 统计或错误信息（含认证失败、冲突拒绝、网络超时分类文案）

### Requirement: 并发与锁定

同步操作 SHALL 全程持有 EventStore 的既有 Mutex（含网络阶段）；同步进行期间业务写命令 SHALL 阻塞等待而非报错。单实例保证（app-shell）确保不存在第二写入进程。

#### Scenario: 同步期间写入阻塞等待

- **WHEN** 同步进行中用户提交记录 Task
- **THEN** 该命令等待同步完成后正常执行，不报错不丢数据

### Requirement: 错误处理与离线可用

同步错误 SHALL 走既有 `{ok: false, error}` 信封并至少区分：网络失败（含超时）、认证失败（提示检查 PAT 权限/有效期）、同 id 内容冲突（点名 event_id）、远端并发更新、未配置。同步不可用（离线/未配置/失败）SHALL NOT 影响任何本地功能。

#### Scenario: 认证失败文案

- **WHEN** 远端返回 401/403
- **THEN** `sync_now` 返回 ok:false 且 error 明确提示检查 PAT

#### Scenario: 离线完全可用

- **WHEN** 无网络环境日常使用
- **THEN** 全部本地功能正常，同步操作返回网络失败信封

### Requirement: 移动端就绪（同步实现约束）

同步实现 SHALL NOT 使用子进程、sidecar 或任何桌面专属运行时能力（遵循 app-shell 移动端就绪约束）；git 协议 SHALL 由纯 Rust 库（gix）在进程内实现，无 C 依赖。

#### Scenario: 依赖审计

- **WHEN** 审计同步相关 Rust 依赖树与源码
- **THEN** 无子进程调用、无 C/C++ 编译目标

### Requirement: 同步设置界面

前端 SHALL 新增 `frontend/js/sync.js`（原生 ES module，零构建、零新依赖），经 `api.js` 既有收口调用同步 command，提供：远端 URL / PAT / 分支的配置表单（保存调 `sync_set_config`）、「同步」按钮（调 `sync_now`）、最近同步时间与结果展示。PAT 输入框 SHALL 以密码形态展示。其余既有前端模块 SHALL 零改动（`index.html` 挂载入口除外）。

#### Scenario: 配置并同步

- **WHEN** 用户在设置面板填写远端信息保存后点击「同步」
- **THEN** 调用链经 api.js 收口完成，面板显示统计结果并刷新最近同步状态

#### Scenario: 既有模块零改动

- **WHEN** 对比变更前后 `frontend/`
- **THEN** 仅新增 sync.js 与 index.html 挂载点，其余 17 个模块与样式不变
