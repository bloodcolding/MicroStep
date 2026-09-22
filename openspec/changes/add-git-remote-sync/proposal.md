# proposal.md · add-git-remote-sync

## Why

事件流目前只存在于单机 AppData 目录（ADR-003：Change 2 前备份责任在用户），换机即丢数据、多设备无法共享成长档案。产品愿景（本地优先 + 移动端路线图）要求同一事件流可跨设备延续。事件溯源的 append-only + 全局唯一 `event_id` 使"集合求并"成为天然的合并语义——本变更按 ADR-003 ④ 既定路线交付 Git 远端同步：远端配置、PAT 凭据、push/pull、union merge。

## What Changes

- **新增能力 `data-sync`**：同步引擎（fetch → 应用层 union merge → 快照 commit → push）、同步配置 `sync.json`（远端 URL / PAT 明文 / 分支 / 最近同步结果）、启动 best-effort pull + 手动同步触发、全程持 EventStore 锁的并发模型、错误信封分类。
- **MODIFIED `ipc-api`**：IPC command 14 → 17，新增 `sync_get_config`（PAT 脱敏回显）/ `sync_set_config` / `sync_now`（返回 {pulled, pushed, merged} 统计）；参数键 camelCase 约定与响应信封 `{ok, error, ...}` 不变。
- **MODIFIED `data-storage`**：数据目录 git 仓库从"预留结构"升级为"同步时写快照 commit"；远端配置持久化于 `sync.json`，不写入 `.git/config`（仓库由应用管理）。
- **依赖**：引入 `gix`（gitoxide，纯 Rust）最小 feature 集（新增 ADR-004，ADR-002 白名单扩展）；不使用子进程/sidecar，app-shell 移动端就绪约束零破坏；前端零新依赖。
- **前端**：新增 `frontend/js/sync.js`（第 18 个 ES module）同步设置面板，经 `api.js` 既有收口调用；其余模块零改动。
- **非 BREAKING**：事件 schema 零变更、既有 14 个 command 契约零变更、未配置同步时应用行为与现状完全一致（离线完全可用）。

## Capabilities

### New Capabilities

- `data-sync`: 数据目录与通用 git 远端（GitHub/Gitee/Gitea 等 HTTPS remote + PAT）的双向同步——union merge 语义（按 event_id 去重、同 id 内容冲突拒绝）、快照式 commit、push 冲突有界重试、bootstrap 三场景、同步设置界面。

### Modified Capabilities

- `data-storage`: 「数据目录 Git 化」要求由"不配置远端、不自动提交的预留结构"修订为"同步时写快照 commit、远端配置位于 sync.json"。
- `ipc-api`: 「IPC command 契约」由"恰好 14 个"修订为"恰好 17 个"（新增 3 个同步 command，既有 14 个逐一保留）。

## Impact

- **代码**：新增 `src-tauri/src/sync.rs`（SyncConfig + SyncEngine）；`commands.rs` / `app_state.rs` 增 3 个信封方法与 command；`lib.rs` setup 增启动 pull spawn；`frontend/js/sync.js` 新增 + `index.html` 挂载入口。
- **依赖**：`gix` 及其传递依赖（feature 裁剪至 fetch/push/HTTP 传输最小集，Cargo.lock 锁定）；无 C 依赖、无子进程。
- **数据**：事件 schema 零变更；`events.jsonl` 追加语义零改动（merge 只追加远端独有行）；数据目录新增 `sync.json`。
- **文档**：README 增同步配置节；DECISIONS.md 追加 ADR-004（gix 引入）；产品方案/前端设计文档相应补段落。
- **风险**：gix push + PAT 认证成熟度（可行性核查进行中，git2-rs 为备选但引入 C 依赖）；PAT 明文存储（以细粒度单仓库 token 缓解并文档化）。
