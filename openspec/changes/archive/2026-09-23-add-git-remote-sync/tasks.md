# tasks.md · add-git-remote-sync

> Step 3/4 执行版（TDD：红阶段 40 例 → GREEN 全绿 119/119，2026-09-24）。

## 1. 依赖与 ADR

- [x] 1.1 gix（open/fetch/blob 读写/http transport）+ git2-rs（push，libgit2 vendored）最小 feature 集引入，`cargo build` 零警告，Cargo.lock 锁定，记录依赖树实测大小（gix 0.87.1：`blocking-http-transport-reqwest-rust-tls` + `sha1`；git2 0.20.4：`vendored-libgit2` + `https`；+111 crate；linker_messages 警告以 `.cargo/config.toml` 统一静默，见 ADR-004）
- [x] 1.2 追加 ADR-004（gix + git2-rs 引入 + ADR-002 白名单扩展），回填 design.md Q1 可行性核查结论与决策（rustls 后端引入 aws-lc/ring C 依赖与规格「唯一 C 目标 libgit2」字面冲突，已在 ADR-004 记录待裁决）

## 2. 同步配置（SyncConfig）

- [x] 2.1 `sync.json` 读写：字段定义、缺省值、部分更新语义（未携带字段不变、pat 空串清除）〔TC-U01~U05〕
- [x] 2.2 IPC：`sync_get_config`（PAT 脱敏末 4 位，≤4 位全掩码）/ `sync_set_config` 信封方法与 command（camelCase 参数键）〔TC-I20〕

## 3. Union Merge 核心（纯函数）

- [x] 3.1 合并算法实现：本地序保留 + 远端独有追加 + 语义相等去重 + 同 id 冲突拒绝 + 无 id 行处理〔TC-U10~U17〕
- [x] 3.2 单测全分支 + 确定性断言（同输入同输出字节）〔TC-U13〕

## 4. Git 传输（SyncEngine）

- [x] 4.1 fetch（gix）：远端分支 tip + events.jsonl blob 读取（无 checkout；空远端 0 refs 短路走推种子路径）〔TC-I01~I03〕
- [x] 4.2 对象写入：blob/tree/commit（双 parent 合流），快照式（仅同步时 commit）〔TC-I04~I06〕
- [x] 4.3 push（git2-rs）：non-fast-forward 有界重试（≤2，push 前 gix 握手复核远端 tip，绕开 git2 0.20.4 空远端 list() UB）+ 超时（30s，工作线程 + recv_timeout）+ 错误分类映射〔TC-I07/E02/U20~24〕
- [x] 4.4 集成测试：本地 bare repo 充当远端——推种子 / 双向同步 / 并发推进重试 / bootstrap 三场景〔19 例含稳定收敛（防顺序乒乓）与持锁证明〕

## 5. 触发链与并发

- [x] 5.1 启动 best-effort pull：setup 阶段线程 spawn，失败记 last_result，不阻塞不弹窗〔TC-I09a~c〕
- [x] 5.2 `sync_now`：完整 pull-merge-push + {pulled, pushed, merged} 统计 + last_sync_at/last_result 更新〔TC-I01/I22〕
- [x] 5.3 持锁语义：同步期间业务写阻塞等待的并发测试〔TC-I10，before_push 钩子注入确定性观测窗口〕

## 6. 前端同步面板

- [x] 6.1 `frontend/js/sync.js`：配置表单（URL/PAT 密码形态/分支 + PAT 留空保持/勾选清除）+ 同步按钮 + 最近结果展示，经 api.js 收口
- [x] 6.2 `index.html` 挂载入口；其余前端模块零改动——**唯一例外 api.js**：同步命令必须经其路由表收口（AGENTS.md「api.js 是唯一 IPC 收口」不变量优先），diff 仅 +3 路由 +3 包装函数，其余 16 模块零改动（diff 证明）；与规格「其余 17 个模块零改动」字面冲突，待用户裁决

## 7. 验收与收尾

- [x] 7.1 全量回归：cargo test 全绿 119/119（既有 79 + 新增 40），零警告（exit=0）
- [ ] 7.2 真实远端手工冒烟：GitHub + Gitee 各至少一次完整双向同步（含错误路径：错 PAT）——**待用户执行**（TC-M02；Gitee/Gitea PAT 兼容性 = design Q2）
- [x] 7.3 文档：README 同步配置节 + PROGRESS.md 更新（归档总结待 ARCHIVE 时落）
