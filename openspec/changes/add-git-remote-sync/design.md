# design.md · add-git-remote-sync

## Context

ADR-003 ④ 将「远端配置 / PAT / push-pull / union merge」划入 Change 2。现状：数据目录已有手工 `.git` 骨架（HEAD/config/objects/refs，无 commits、无远端）；事件流单文件 `events.jsonl`（append-only JSONL，每事件含全局唯一 `event_id`——纳秒时间戳 + 进程内单调计数）；app-shell 规格硬约束移动端就绪（禁子进程/sidecar）；ADR-002 依赖白名单由 Cargo.lock 锁定。Step 1 Brainstorming 五问落定：多设备双向同步 / 通用 git remote / gix 纯 Rust / 启动 pull + 手动同步 / AppData 明文 PAT。

## Goals / Non-Goals

**Goals**

- 双向同步：任意两台设备离线各自追加后同步，事件流按 union merge 合一，数据不丢不重
- 通用 git remote（HTTPS + PAT），不绑死 provider
- 移动端就绪：进程内纯 Rust git 协议，零子进程、零 C
- 离线完全可用：同步是纯增量能力，未配置/失败不影响任何本地功能
- merge 确定性：同输入同输出，可 golden 式回归

**Non-Goals**

- 不做周期自动同步（ticker 不参与）
- 不做 git merge 机器 / 分支管理 / 冲突交互界面（同 id 内容冲突 = 拒绝并报错）
- 不做凭据库（keyring）存储
- 不做远端 provider 专属 API（REST 文件读写）
- 不做用户手动 git 操作支持（数据仓库由应用管理）

## Decisions

### D1. gix（gitoxide）作为 git 协议实现（排除子进程 git / git2-rs / REST API）

子进程 git 违反 app-shell 移动端就绪且依赖用户环境；git2-rs 引入 libgit2 C 编译，移动端交叉编译多一层复杂度；REST API 绑死单一 provider。gix 纯 Rust、Cargo 自身在用、支持 HTTPS fetch/push + 凭据回调。feature 裁剪至最小集（open repo / fetch / blob 读写 / push / http transport），Cargo.lock 锁定，随变更追加 ADR-004。风险与验证见 Open Questions Q1。

### D2. git 仅作传输与快照历史，merge 在应用层（排除 git merge 机器）

同步正确性 =「事件集合求并」，语义完全由我们掌控、可穷举测试；gix merge 机器 API 面大且引入 git 冲突概念与事件语义双轨。git 在本设计中只承担：fetch（拿远端 blob）、对象写入（blob/tree/commit）、push（更新远端 ref）。

### D3. Union merge 语义：本地序保留 + 远端独有追加 + 按 event_id 去重 + 同 id 冲突拒绝

append-only 红线决定永不改写既有行；跨设备因果可见性由同步本身建立（先拉到 TASK_CREATED 才可能记 TASK_COMPLETED），「本地序 + 追加序」天然满足事件因果拓扑。去重比较用解析后语义相等（容忍跨版本键序/空白差异），避免假冲突；同 id 内容不同 = 事件被改写 = 违反不可变红线，整次拒绝、点名 event_id、双端原样保留。

### D4. 快照式 commit（排除逐事件 commit）

commit 仅在同步时产生，git 历史 = 同步点快照。事件级审计已由 JSONL 本身承载，逐事件 commit 只增加写放大与历史噪音。merge 合流以双 parent commit 记录，保留跨设备合流脉络。

### D5. sync.json 明文 PAT（排除 keyring / 每次输入）

威胁模型：能读 AppData 的攻击者已能直接读 events.jsonl 本体，PAT 的增量风险仅为远端写权限——以文档建议细粒度单仓库 contents 读写 token 收敛爆炸半径。keyring 引入新依赖且 Linux 无 secrets 环境会降级失败；每次输入体验不可接受。与 git 自带 credential store 同安全水位。

### D6. 同步全程持 EventStore Mutex（排除无锁快照 + 两阶段重合并）

个人应用同步秒级（单文件百 KB 级 + 两次网络往返），持锁（含网络阶段）杜绝「快照后用户又追加」竞态，业务写命令阻塞等待而非报错。单实例插件保证无第二写入进程。

### D7. 触发 = 启动 best-effort pull + 手动 sync_now（排除周期自动同步）

启动 pull 保证开机看到最新（异步 spawn、失败仅记 last_result，不阻塞不弹窗）；完整 push 由用户显式触发，心智模型简单、网络失败无需重试风暴。周期同步留待真实使用反馈再评估。

### D8. .git 由应用管理，远端不写入 .git/config

远端配置只存 sync.json，.git/config 保持应用自管理状态，避免用户手动 git 操作与 union merge 心智模型冲突（手动 merge 产生的历史会被应用侧快照 commit 覆盖语义）。灾难恢复文档化：手工 clone 远端 → 拷贝 events.jsonl。

### D9. 测试远端 = 本地 bare repo（file/path 传输，零网络）

集成测试以临时 bare repo 充当远端（gix 原生支持本地路径 remote），覆盖推种子 / 双向同步 / 并发推进重试 / 快照 commit。诚实边界：file 传输无认证面，401/403 只能单测错误映射 + 真实远端手工冒烟。merge 纯函数全分支单测 + 确定性断言（同输入同输出字节）。

## Risks / Trade-offs

- **gix push + PAT 成熟度**（Q1 核查中）：若核查或实现证明 push/认证不可用，fallback 到 git2-rs 需重估移动端约束并回到设计门禁；子进程 git 为最后手段（需 MODIFIED app-shell 规格）。
- **依赖树膨胀**：gix 最小 feature 集仍有数十传递 crate；以 Cargo.lock 锁定 + ADR-004 记录，换取零 C 零子进程。
- **PAT 明文**：接受（D5），文档建议细粒度 token + 定期轮换。
- **同步期间 UI 写入阻塞**：秒级持锁可接受；若未来数据量增长至同步数十秒，需引入同步进度反馈或拆锁。
- **同 id 冲突拒绝的恢复路径**：属人工修复场景（理论上是 bug 或手改文件才会触发），错误信息给出 event_id 引导排查，不提供自动解决。

## Migration Plan

- 无事件 schema 变更，存量数据目录零迁移：首次同步走 bootstrap 空远端推种子路径。
- 回滚：删除 `sync.json` 即回到现状（同步未启用）；`.git` 内快照 commit 不影响应用读写；gix 依赖移除 = 还原 Cargo.toml/lock。
- 发布顺序：先 `cargo test` 全绿（本地 bare repo 集成测试），真实远端冒烟（GitHub + Gitee 至少各一）后合入。

## Open Questions

- **Q1（已核查，2026-09-23 回填）**：gix 可行性结论——**fetch 侧 FEASIBLE / push 侧 BLOCKED**。fetch + blob 读取 + 对象写入 + ref 更新全部生产可用：Cargo 内置 `fetch_with_gitoxide`（`-Zgitoxide`）、GitButler（同为 Tauri 应用）以 `blocking-http-transport-reqwest-rust-tls` 纯 Rust feature 组合生产运行；PAT 走内存 Basic 认证（`set_identity`/`with_credentials`），`http.extraHeader` 支持 Bearer。**但 push 在 gix 各层（gix / remotes / gix-protocol）均未实现**（官方 crate-status 全层未勾选；维护者 Byron 2026-02 明确 "doesn't even support pushes yet"）；jj 以 git 子进程做 push、GitButler 退回 git2、Cargo 从不 push，无任何第三方纯 Rust push 客户端基于 gix。**push 必须三选一**：(a) 基于 gix-transport packetline + gix-pack thin pack 自研最小 send-pack 客户端（原语齐全、单 ref 无 force/atomic 场景有界，但无文档无先例、成为自维护协议代码）；(b) push 退回 git 子进程（违反 app-shell 移动端就绪字面约束，需 MODIFIED 规格）；(c) push 退回 git2-rs/libgit2（进程内 C 库不违 app-shell 字面——仅违本设计稿"无 C 依赖"自加码条款；代价 = 双 git 栈 + C 构建）。决策待用户，设计门禁重开。
- **Q2（冒烟阶段）**：Gitee/Gitea PAT 认证兼容性（用户名形态、token 前缀要求）以真实远端实测为准，文档记录 provider 差异。
