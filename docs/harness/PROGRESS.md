# PROGRESS.md · 进度与待办

> **使用规则**
> - 每次会话开始（上班打卡）先读本文件 + `DECISIONS.md`。
> - 每完成一个可验证步骤立即更新本文件；下班前必须更新 Next Steps。
> - 完成的条目移入归档区并附简短总结；超过 500 行时将旧条目移入 `docs/harness/archive/`。

## 当前状态

- **日期**: 2026-09-25
- **健康度**: ✅ cargo test 119/119、exit=0；cargo build 零警告；openspec validate --all --strict 8/8 通过（含 frontend-i18n）；实机隔离冒烟 36/36 全过
- **阶段**: **Change 4（add-frontend-i18n）已归档**（frontend-i18n 新能力落账，8 个主规格）
- **数据档案**: 全新事件流（ADR-001）；数据落 `%APPDATA%\com.microstep.app`（ADR-003）；仓库内 `data/events.jsonl` 为 Python 时代历史存档，应用已不读，仅 Git 历史保留

## Next Steps

1. **Gitee 冒烟（可选，design Q2）**: 第二 provider 完整双向同步，记录 PAT 用户名形态差异
2. 日常使用期：真实数据积累于 `%APPDATA%\com.microstep.app`，装机用 `npx -y @tauri-apps/cli build` 产物

## Suspended Tasks（暂存任务区）

_（无。任务切换时将未完成工作记入此处，向用户确认后切换。）_

## 归档区（结项总结）

### 2026-09-25 · add-frontend-i18n 结项：前端运行时多语言（zh-CN / en）

- 四步法收官：TEST_PLAN（TC-U01~U12 / TC-I01~I14 / TC-E01~E04）→ 静态红基线 10/10 FAIL → 最简实现 → CDP 实机冒烟 36/36 + cargo test 119/119 全绿后归档；`openspec archive` 已落账 frontend-i18n 7 Requirement（主规格 8/8 validate --strict 通过）。
- 交付：`frontend/js/i18n/`（index.js + 双字典 177/182 键，基名一致 + en `.one` 复数 5 组）；`t()` 插值/复数/zh-CN 回退 + AGENT-I18N warn；顶栏语言切换（两视图、零 IPC、localStorage 持久化、navigator 检测）；index.html 全量 data-i18n 标记（含 title/placeholder/aria/html lang）；19 个模块动态文案机械提取；dimensionName/fullName 字典优先三级兜底；数字/日期 locale 跟随；设置面板与业务模态打开态切换重建并保留 tab/表单值；Rust/IPC/schema/依赖零改动。
- 验证方法论：无前端测试基建 → 静态审计（session 级命令，未入库）+ Node DOM-stub 模块图冒烟 22/22 + WebView2 CDP 实机冒烟（隔离 identifier + `--lang` 多轮启动控制 navigator.language；Emulation.setLocaleOverride 不影响它，首版踩坑）；冒烟抓到 modals.js 订阅器缺 `$` 导入的运行时 ReferenceError。
- 关键口径：api.js 仅改 import + 两条前端兜底错误文案（R5 要求；路由表/invoke/IPC 契约零改动）；事件条目标题 `event.name`（后端生成嵌用户数据）按 R7 原文直显，类型 chip（前端生成）本地化；zh 视图类型 chip 由原始枚举变为中文描述。

### 2026-09-25 · add-settings-panel 结项：统一设置面板 + 同步入口迁移

- 四步法收官：TEST_PLAN（24 个 TC-SP + 文档 / 回归 / OpenSpec 项）→ 静态红基线（settings.js 缺失、旧同步卡仍在、sync.js 自绑定）→ 最简实现 → 实机隔离冒烟 + 119/119 全绿后归档；`openspec archive` 已落账 settings-ui 7 Requirement，并修订 app-shell / data-sync。
- 交付：顶栏「⚙ 设置」两视图可达；宽体模态内恰好三类 tab（代码仓同步 / 数据档案 / 关于）；sync.js 改为导出表单 + 绑定 + 回显，由 settings.js 挂载；仪表盘旧同步卡整卡移除；面板全 div 结构规避 modalBody 统一 submit 代理。
- 验证：隔离 identifier + 本地 bare 远端实机冒烟覆盖两视图入口、三类 tab 零请求、PAT 保存 / 留空保持 / 勾选清除、同步前自动保存、保存失败中止、同步成功与错误 toast、旧卡无残留；Rust / IPC / schema / 依赖零改动。
- 关键难点：① 规格文案写 `state.events`，实机冒烟暴露真实字段为 `state.event_stream`，已修复事件总数；② 设置面板请求返回前可能被关闭，回显需先确认 DOM 仍存在，避免 null 赋值变成错误 toast；③ Tauri core.invoke 只读，CDP 观测需在页面导航前替换 core 对象拦截。

### 2026-09-25 · 同步冒烟收尾确认：GitHub 双向同步用户实证通过

- 用户在应用内完成最终同步确认（Next Steps 1 闭环）：真实 GitHub 远端双向同步 OK，add-git-remote-sync 遗留的手工冒烟（TC-M02/tasks 7.2）至此全部完成。
- 401 文案已在真实场景实证（首次未保存 PAT 即同步命中），修复（`964ad57`）后通路正常。
- 安全项闭环：旧 PAT 已由用户在 GitHub 手动撤销重建（2026-09-25 确认），泄露风险解除。

### 2026-09-24 · add-git-remote-sync 结项：Git 远端同步（gix fetch + git2 push 双栈）

- TDD 全程：TEST_PLAN（36 TC 编号）→ 红阶段 40 例（4 测试文件；本地 bare repo 充当远端 D9；`SyncHooks.before_push` 钩子把「fetch 后远端被推进」与「同步持锁」两类并发场景做成确定性注入）→ GREEN 一次贯通，119/119 全绿零警告，openspec validate --strict 通过后归档。
- 交付：`src/sync.rs`（SyncConfig + union_merge 纯函数 + SyncEngine）、AppState 3 信封、IPC 14→17、启动 best-effort pull、前端 `sync.js` 面板（PAT 密码形态/留空保持/勾选清除）；ADR-004 落账（gix 0.87 + git2 0.20，+111 crate）。
- 关键难点：① 稳定收敛防乒乓——事件集合一致且内容与上次 commit 相同时零动作（否则跨设备行序差异会导致无限互推 merge commit）；② 空远端 fetch 在 gix update_refs 阶段报 NoMapping，须以 advertisement 0 refs 短路走推种子路径；③ git2 0.20.4 `Remote::list` 在 0 refs 空远端触发空指针 UB 检查崩溃，push 前远端复核改用 gix 握手（prepare_fetch 不 receive）；④ MSVC 链接 C 对象使 link.exe stdout 被 rustc 捕获为 linker_messages 警告，以 `.cargo/config.toml` 统一静默（探针对照确认为纯链接器通告）。
- 已知规格字面偏差（用户「继续」放行，ADR-004 记录在案）：rustls 后端引入 aws-lc/ring C 依赖（纯 Rust TLS 无成熟替代）；api.js 加 3 条路由维持「唯一 IPC 收口」（其余 16 模块零改动）。
- 遗留：tasks 7.2 真实远端手工冒烟（TC-M02）由用户结项后执行。

### 2026-09-24 · 真实远端冒烟（结项后跟进）：401 根因 = UX 陷阱，通路实证通过

- 用户应用内首同步报「认证失败（401）」：读 sync.json 定位 `pat: ""` —— 表单值不自动落盘，未点「保存配置」即同步。错误分类逻辑本身正确（401 → 认证失败 ✓）。
- 修复（`964ad57`）：「立即同步」前自动保存表单，保存失败中止；重建重启应用。
- 真实 GitHub 通路实证（隔离临时分支 + 自动清理 + 临时测试脚本跑完即删）：推种子 pushed=1 → 双向拉取 pulled=1 → 分支清理成功。PAT 有效、gix fetch + git2 push 在 HTTPS 真实远端全通。
- 关键难点：运行中的 `cargo run` 会锁 `target\debug\microstep.exe`，cargo test 重建 bin 前必须先停应用（os error 5 拒绝访问）。

### 2026-09-22 · refactor-to-tauri-v2 收官：tasks 6.2 Python 退役（22/22 全完成）

- 删除 `backend/`（domain/store/server）、`tests/`（4 个 Python 测试/脚本）、`run.py`、`start.bat`、`pyproject.toml`（纯 Python 工程清单，随退役删除，Git 历史保留）；`data/events.jsonl` 为历史用户数据，保留不动。
- AGENTS.md 对齐 Tauri 形态：命令速查表（cargo run/build/test + npx tauri build）、架构图（src-tauri 六模块 + AppData 落盘）、真实数据保护改为 AppData 口径、调试规范改 Rust eprintln/前端 console.log、新增 IPC 参数键 camelCase 不变量（冒烟踩坑沉淀）。
- ERRORS.md ERR-001 状态闭环（批次一已修复，Python 测试随退役消亡，Rust tests/domain.rs 承接）。
- 退役后回归：cargo test 79/79 全绿；openspec validate --strict 通过。
- 关键难点：无——门禁（golden 全绿 + Windows 构建实测 + 手工冒烟三链路）在 6.1/6.3 已全部满足，删除动作本身零风险；唯一取舍是 pyproject.toml 超出 tasks 6.2 字面清单（保留它将与新 AGENTS.md 自相矛盾，故随退役一并删除）。

### 2026-09-22 · 手工冒烟三链路全绿 + api.js camelCase 参数修复（tasks 1.3/6.1 完成）

- 冒烟方法：`WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS=--remote-debugging-port=9222` 启动 Tauri 应用，Playwright 经 CDP 接入 WebView2 实机点击（非 HTTP 模拟）。
- 三链路全过：① 建 Task（TASK_CREATED 落 AppData，effects/repeatable/epic_id 字段正确，UI 同步）② 记录结算（TASK_COMPLETED 落盘，知识 0→10，bonus 0%）③ 事件删除（EVENT_DELETED tombstone，原事件保留，属性不回滚）；仓库 `data/` 全程零污染（git status 验证）。
- **发现真 bug 并修复**：点击「＋」记录静默失败（toast 稍纵即逝 + 控制台零输出）。探针实证根因：Tauri v2 命令参数键默认 camelCase（`task_id`→需传 `taskId`），api.js 按 snake_case 传参——必填参数（task_id/epic_id/event_id）invoke 直接 reject；可选多字参数（epic_id/title_id/title_bonus_percent）静默变 None。修复：api.js 在 invoke 前对顶层参数键统一 snake→camel 转换（嵌套值透传），其余 16 个前端模块零改动。
- 回归：cargo test 79/79、Python 44/44 + smoke 全绿；graphify 已同步。
- 关键难点：Tauri v2 前端资源编译期内嵌，改前端必须 `cargo build` 重建重启（无热重载）；单字参数键（title/effects/note）不受影响，故建 Task 等链路此前未暴露此 bug。

### 2026-09-22 · Tauri v2 重构批次二结项：壳层接线 + IPC 信封层 + api.js 透明适配

- 壳层五件套：`app_state.rs`（AppState{Mutex<EventStore>} + 14 信封方法，逐字段镜像 server.py 强制转换）、`commands.rs`（14 个 async command）、`data_dir.rs`（AppData + 手工 .git 骨架 HEAD/config/objects/refs，无子进程满足移动端就绪约束）、`ticker.rs`（tokio sleep 循环，先结算后等待 + catch_unwind 不中断）、`lib.rs`/`main.rs` 真接线（single-instance 最先注册 → setup AppData 初始化 + 启动补结算 → ticker）。
- Python 怪癖全数复刻并测试钉死：显式 null→"None"、`x or 默认` 假值回退、float 失败文案（ValueError 原文 / TypeError 加「服务器内部错误:」前缀）、effects 非列表视为缺省、**KeyError 的 str() 是 repr → error 带单引号**（如 `"'Task 不存在'"`）、update 的 epic_id 缺省=移出里程碑 / repeatable 显式 null=置 false。
- api.js（26→64 行）invoke 适配：精确表 + 参数段匹配，/complete 别名→log_task，decodeURIComponent，导出签名不变；`git diff --stat frontend/` 证明仅 api.js 单文件变化（其余 16 模块零改动）。
- 下班 Check：cargo test 79/79（新增 commands 22〔TC-I01~I21 + TC-E14 八线程并发〕+ data_dir 3 + ticker 2）、cargo build 零警告、Python 44/44 + smoke 全绿、graphify 已同步。
- 关键难点：① Tauri 约束 async command + 借用参数（State<'_>）必须返回 Result（Err 分支永不使用）；② Python `manual_tick()` 实际只做 ensure_ready（返回的 tick 字典被丢弃不落盘），system_tick 信封据此实现；③ tokio 需显式补 `rt` feature 供测试。

### 2026-09-22 · Tauri v2 重构批次二里程碑：脚手架 + 依赖栈编译验证（下班收口）

- 脚手架五件套就位并通过 tauri-build 编译校验：Cargo.toml（tauri 2.11.6 / tauri-build / single-instance / chrono / tokio + [[bin]] + release profile）、build.rs、tauri.conf.json、capabilities/default.json、icons/（tools/gen_icons.py stdlib 生成 32/128 PNG + ICO）。
- domain.rs 补 dimension_meta / effect_dimensions / title_bonus_dimensions（/api/meta 下发口径）；store.rs 落位 real_clock（chrono 本地时区，等价 Python date.today/now_iso）+ EventStore::open；main.rs 占位使 [[bin]] 可解析（真接线待批次二续做）。
- 下班 Check：cargo test 52/52 全绿（首次拉取并编译 tauri 依赖栈，467 包锁定，1m23s，走 7897 代理）；Python 44/44 + smoke 全绿；ADR-002（技术栈切换 + 依赖统一引入）已落账。

### 2026-09-22 · Tauri v2 重构 Step 3+4 批次一：golden 基准 + Rust 领域核心行为等价

- ERR-001 修复（选项 A，测试钉死固定日期）；Rust 工具链安装（rustup 1.98.1 + MSVC 14.44 + WebView2，走本地 7897 代理；VS 安装器 `--proxy`/`--wait` 为引导器专属参数，setup.exe 不认）。
- golden 基准资产 4 份入库（真实流 18 事件 7 类型 + 构造序列 23 事件 14 类型，双跑字节一致）；红阶段 52 例（26+18 移植 + 8 新增护栏，TC 编号可追溯）。
- GREEN 关键难点 ①：Python `clamp(v, 0, 100)` 位置参数 int 边界——`min(100, 100.0)` 平局返回 int 100，SAN 触顶/触底时历史与维度值变 int（JSON "100" vs "100.0"）；以 `serde_json::Number` 建模 + `py_clamp_number` 复刻，真实流三天顶格 100 暴露、已钉死。②：serde flatten+tag 下 Option 把显式 null 折叠为缺省（unlock_title_id tri-state），用 `deserialize_with` 直通 `Value::deserialize` 绕过。
- 已知良性差异（corpus 外，代码注释已记）：today() 暂用 UTC 日期（本地时区真实时钟随 Tauri 壳批次）；事件文件新增行的数字字面量（int 10 vs 10.0）与键序按 Rust 侧书写。

## 归档区（结项总结）

### 2026-09-22 · Tauri v2 重构 Step 1+2：设计批准 + OpenSpec 提案

- 四项关键决策（苏格拉底式提问落定）：先桌面三端/移动进路线图 → 后端 Rust 化（sidecar 出局）；Python 彻底退役；AppData 落盘 + Git 同步路线（Change 2）；两阶段两个变更交付。
- 产出 `openspec/changes/refactor-to-tauri-v2/`：proposal（3 项 BREAKING + 5 能力）、5 个规格增量（15 Requirement / 27 Scenario）、design（D1-D8 决策 + 迁移/回滚计划）、tasks（6 组 22 项占位）。`openspec validate` + `--strict` 双通过。
- 核心方法论：golden replay 对照移植——事件溯源纯函数 Reducer 让「同一事件流 Python/Rust 重放 State 零差异」成为免费的完美等价性验收。
- 下班 Check 发现 ERR-001（预先存在的时钟敏感测试，与本会话无关，已登记待决策）。

### 2026-09-20 · start.bat 一键启动 + 发布 tag 0.0.1

- 新增 `start.bat`（纯 ASCII，规避批处理中文编码坑）：`cd /d %~dp0` 保证任意目录双击可用；python 存在性检查；后台 helper 延迟 3 秒开浏览器（等服务绑定端口）；README 运行节补充一键启动说明。
- 实测通过：隔离数据（`MICROSTEP_EVENT_PATH`）启动 → `/api/state` 200 → 杀进程清理，真实 `data/events.jsonl` 零污染（git status 验证）。
- 提交后打 annotated tag `0.0.1` 并推送 origin master。

### 2026-09-19 · 路线 B 收官：路由表驱动 + 校验收敛 + 事件类型化

- 新增 tests/test_server.py（24 例，真实 ThreadingHTTPServer + urllib），先建安全网再动刀；期间发现并修复真实 bug：非法 JSON 请求体在 try 块外未捕获导致断连，现统一 400。
- server.py 140 行 if-链 → 三张路由表（GET 精确 / POST 精确 / POST {param} 段匹配）+ 模块级 handler，统一签名 (store, body, params)。
- 校验收敛 store 层：_validate_epic_fields 共用于创建/更新；铭文 ≥2 字、Task/Epic 标题非空下沉；server 只留参数解包。
- 事件 schema TypedDict 化：14 种事件（Literal 标签），11 个工厂返回类型 + 12 个 Reducer handler 参数收紧，零运行时开销。
- 提交：9b7c24c / c5a0bf6 / 62b28c2 / 55a02b2。
- 关键难点：测试安全网先行使重构全程绿灯（期间两次编辑失误均被测试即时捕获）。

### 2026-09-19 · 重构第一批：前端模块化 + 路线 A 还债

- 前端 app.js（917 行）拆为 17 个原生 ES modules（js/ + js/render/），无构建步骤，列表交互改事件委托消除渲染层反向依赖；浏览器 QA 全过（加载/建 Task/觉醒/雷达/结算）。
- 新增 tests/test_domain.py + tests/test_store.py（42 例，stdlib unittest），期间抓到一次误删校验行的编辑事故，安全网价值实证。
- 修复前端 SAN 提示文案谎言（原描述不存在的疲劳收益惩罚机制）；产品方案.md 重构为「实现现状 + 规划蓝图」；前端设计.md 从空文件补全。
- EventStore：read_events 按 mtime 缓存（单命令从 ~6 次文件读取降到 1-2 次）；15 处 ensure 样板收敛为 _ensure_ready；MAX_EQUIPPED_TITLES 常量经 /api/meta 下发；消除 import backend.server 即创建数据文件的副作用。
- 提交：0edd3e6 / f55400c / 350d1b1 / 3dac05e / ee0ba3b。
- 关键难点：ES modules 拆分的循环依赖（actions↔controller↔render）用容器级事件委托破环；mtime 缓存需保证外部直接改文件时自动失效。

---

## 格式约定

- **当前状态**：一句话级别的事实（健康度 / 阶段 / 阻塞点），不放细节。
- **Next Steps**：可直接执行的下一步，每条带验证点。
- **Suspended Tasks**：被中断的任务，记录断点（改到哪个文件、测试状态、下一步）。
- **归档区**：每个结项一条，格式 `### YYYY-MM-DD · 任务名` + 3-6 行总结。
