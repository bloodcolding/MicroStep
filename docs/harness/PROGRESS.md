# PROGRESS.md · 进度与待办

> **使用规则**
> - 每次会话开始（上班打卡）先读本文件 + `DECISIONS.md`。
> - 每完成一个可验证步骤立即更新本文件；下班前必须更新 Next Steps。
> - 完成的条目移入归档区并附简短总结；超过 500 行时将旧条目移入 `docs/harness/archive/`。

## 当前状态

- **日期**: 2026-09-22
- **健康度**: ✅ cargo test 52/52 全绿（tauri v2 依赖栈 467 包锁定并编译通过，零警告）；Python Check 全绿（44/44 + smoke）
- **阶段**: refactor-to-tauri-v2 Step 4 批次二进行中——脚手架五件套 + domain meta + store 真实时钟已就位并通过编译验证（ADR-002 已落账）；剩 app_state/commands/data_dir/ticker/main 接线 + api.js 适配
- **数据档案**: 全新事件流（历史数据已清空，见 ADR-001）；Tauri 迁移后数据将迁至 AppData（ADR-003 待 tasks 6.3 落账）

## Next Steps

1. 批次二续做：`src-tauri/src/app_state.rs`（AppState{store: Mutex<EventStore>} + 14 个信封方法，严格镜像 backend/server.py 的 str()/or/isinstance 强制转换语义，含 Null→"None" 怪癖）
2. `data_dir.rs`（init_data_dir + 手工 .git 骨架，无子进程）+ `ticker.rs`（600s tokio interval + ensure_daily_ticks，测试可变时钟）+ `commands.rs`（14 个 #[tauri::command] 薄封装）+ `main.rs` 真接线替换占位（single-instance 首个注册 + setup + ticker）
3. 新增 tests/commands.rs（TC-I01~I21 + TC-E14 八线程并发）、tests/data_dir.rs（TC-I26~I28）、tests/ticker.rs（TC-I29/I30 可变时钟跨日）
4. api.js invoke 适配（保持 api(path, options) 导出签名，路径→命令映射含 /complete 别名→log_task）；`git diff --stat frontend/` 验证仅 api.js 变化
5. 全量验证（cargo test + cargo build + Python Check）后提示用户手动 commit；tasks 6.3 收尾时 README 重写 + ADR-003（AppData 迁移）落账

## Suspended Tasks（暂存任务区）

_（无。任务切换时将未完成工作记入此处，向用户确认后切换。）_

## 归档区（结项总结）

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
