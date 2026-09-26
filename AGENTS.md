# AGENTS.md · MicroStep 2.0

本仓库是 **Tauri v2 桌面应用形态的事件溯源个人成长 RPG**（本地优先；Rust 事件溯源内核 + 原生 JS 单页前端，前端无构建步骤）。Rust 侧依赖由 `src-tauri/Cargo.lock` 锁定白名单（ADR-002）；数据落系统 AppData（ADR-003）。Python 版已由 `refactor-to-tauri-v2` 变更完成行为等价移植并退役（golden replay 零差异验收）。产品愿景见 `产品方案.md`，IPC 命令全表见 `README.md`。

## 1. Agent 核心工作流 (Standard Operating Procedures)

### 功能开发：四步法（设计 → 规格 → 测试 → 实现）

详细流程见 📖 [DEVELOPMENT_FLOW.md](DEVELOPMENT_FLOW.md)。快速概览：

- **Step 1** Superpowers Brainstorming → 输出 `DESIGN_SUMMARY_FOR_OPENSPEC`（门禁：用户回复 `APPROVED`）
- **Step 2** OpenSpec Propose → 规格文档（门禁：`APPROVED` + `openspec validate`）
- **Step 3** Superpowers TDD → 测试方案（门禁：`TEST_PLAN_APPROVED`）→ 失败测试代码（门禁：`CONTINUE`）
- **Step 4** 最简实现 + **全量回归** + 验证报告（门禁：`ARCHIVE`）

> ⚠️ **强制纪律**：会话隔离（Step 1+2 一个会话，Step 3+4 新会话、只加载规格）；每步需用户明确批准；实现中发现规格矛盾立即停止编码、请求用户决策。

### 日常维护（简单 bug 修复 / 配置调整 / 文档更新，无需四步法）

**每次会话开始（上班打卡）**：
1. 读取 `docs/harness/PROGRESS.md`（进度与待办）和 `docs/harness/DECISIONS.md`（历史决策，避免走弯路）。
2. 若用户需求模糊：先输出理解（1-3 句）+ 关键假设（2-6 条），确认后再继续。

**编码与验证循环（阶段打卡）**：
1. 测试通过即暂存：一旦某功能的测试跑通，**立即 `git add .`**。
2. 完成相对完整的逻辑闭环 → WIP 提交：`git commit -m "chore(wip): [模块名] 跑通 xxx 测试/完成 xxx 逻辑"`。
3. 下一步修改前用 `git diff --staged` 自检，确保没有夹带无关改动。
4. 变更确认无误后执行 `graphify update .` 同步 AST 知识图谱。

**任务切换**：将未完成工作记入 `PROGRESS.md` 的 Suspended Tasks 区域，向用户确认后切换。

**会话结束前（下班打卡）**：
1. 运行 Check 命令（见速查表）且**全部通过**。失败时只修与本次改动直接相关的；仍失败 → `git stash` 暂存 → 记录 `docs/harness/ERRORS.md` → 向用户求助。
2. 更新 `PROGRESS.md`（标记完成、更新 Next Steps）。
3. 重大架构调整 / 技术选型 / 踩坑 → 按 ADR 格式追加 `DECISIONS.md`。
4. **提示用户手动 commit（Agent 不自动 push）**；仅当用户明确要求时才代为 commit。

**任务全部完成（结项）**：清理调试代码 → 必要时同步 README 等文档 → 在 `PROGRESS.md` 归档区写总结（含关键难点）→ 跑全量测试确认无回归。`PROGRESS.md` / `DECISIONS.md` / `ERRORS.md` 超 500 行时，将旧条目移入 `docs/harness/archive/`。

**异常处理与恢复**：同一错误修复尝试 **≤2 次**，仍失败禁止重试 → 记录 ERRORS.md → 请求用户指导。会话意外中断后：读 PROGRESS.md + ERRORS.md → 检查最近成功构建的 commit → 向用户汇报断点并确认是否继续。

## 2. 项目专属上下文 (Project Specific Context)

> 以下为本项目特定环境、架构和命令，严格遵循。

### 核心命令速查表 (Cheat Sheet)

| 用途 | 命令（Windows PowerShell） |
| --- | --- |
| 启动应用（开发窗口） | `cargo run`（在 `src-tauri\` 下执行） |
| 发布构建 | `cargo build --release`（产物 `src-tauri\target\release\microstep.exe`） |
| 打安装包 | `npx -y @tauri-apps/cli build`（产物 `src-tauri\target\release\bundle\`，需 Node） |
| 全量测试 = 唯一 Check 命令 | `cargo test`（在 `src-tauri\` 下执行；121 例含 golden 回归） |
| CI 回归 | push master / PR 自动触发 `.github/workflows/ci.yml`（cargo test/build --locked + Android 目标 check） |
| 全平台发布 | 打 tag `v*` 触发 `.github/workflows/release.yml`（版本守卫 → 五平台产物 → 草稿 Release） |

- `cargo test` 中 store/commands/data_dir 等测试均用 tempdir 隔离，不触碰真实事件流；golden 回归以 Python 版导出的 State 快照为基准资产，行为漂移会被立即捕获。
- 本仓库**没有** lint / typecheck / formatter 配置，不要自行引入；CI 仅有 `.github/workflows/{ci,release,mobile-gen}.yml`（本地全量 Check 仍是 `cargo test`，CI 是推送后的补充回归；不要新增其他流水线）。"检查通过" = `cargo test` 全绿（+ `cargo build` 无新增警告）。
- `cargo run` / `cargo build` 是挂起或长命令：必须后台运行或提示用户手动执行，禁止阻塞主对话终端。
- 前端资源在**编译期内嵌**：改前端必须 `cargo build` 后重启应用（无热重载）；改 Rust 同样需重启。
- WebView2 可开 CDP 调试：设 `WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS="--remote-debugging-port=9222"` 后启动，即可用浏览器自动化工具接入实机 UI（冒烟实测方法）。

### 架构与硬性不变量

```text
src-tauri/src/domain.rs     纯函数 Reducer（apply_event/build_state）+ 维度/称号定义 —— 全部业务规则在此
src-tauri/src/store.rs      EventStore：JSONL 事件流读写 + 业务命令（Mutex 保护，损坏行容错跳过）
src-tauri/src/app_state.rs  AppState：14 个信封方法（Python 版强制转换语义逐字段镜像，错误都在信封内）
src-tauri/src/commands.rs   14 个 #[tauri::command] IPC 薄封装（恰 14 个，无多余）
src-tauri/src/data_dir.rs   AppData 数据目录初始化 + 手工 .git 骨架（无子进程）
src-tauri/src/ticker.rs     tokio 周期任务：启动补结算 + 跨零点补每日结算
frontend/                   原生 JS 单页应用（17 个 ES modules，无构建步骤）；js/api.js 是唯一 IPC 收口
%APPDATA%\com.microstep.app\events.jsonl  唯一持久化：append-only 事件流（真实用户数据，不纳入本仓库 Git）
```

- **依赖白名单**：Rust 侧依赖由 `Cargo.lock` 锁定（tauri/serde/chrono/tokio 等，ADR-002）。引入任何新依赖（含传递依赖）必须先征得用户同意并追加 ADR；前端保持零构建、零 npm 依赖。
- **事件不可变**：绝不改写或删除事件流中的行；"删除" = 追加 `deleted` 标记事件（tombstone）；Task 只是事件流的投影，删改 Task 不回写历史事件。修改业务规则 = 修改 `domain.rs` 的 Reducer，重放同一事件流即完成规则升级。
- **真实数据保护**：AppData 下的 `events.jsonl` 是用户真实数据，禁止手工改写做实验；自动化验证一律走 `cargo test`（tempdir 隔离）。仓库内 `data/events.jsonl` 是 Python 时代的历史存档，应用已不读它，仅作 Git 历史保留。
- **维度白名单**：`san, physical, professional, knowledge, expression, kindness, charm`。**不存在** willpower / EXP / 等级体系 —— 测试断言其不存在，禁止重新引入。
- **数值规则**：SAN 是日槽（clamp 0–100，每日从 100 重置，扣减超过当前 SAN 时后端拒绝结算）；其余六维为 pool（clamp ≥ 0）。称号加成按 `1 + title_bonus_percent / 100` 结算，最多装备 3 个。
- **IPC 参数键 camelCase**：Tauri v2 命令参数键为 camelCase（`task_id` → `taskId`），`api.js` 已做统一转换；新增 command 或改参数时勿破坏此约定（冒烟曾在此翻车）。
- IPC 命令全表与请求体示例见 `README.md`；产品口径见 `产品方案.md`；前端交互设计见 `前端设计.md`；变更规格见 `openspec/`。

### 知识库索引 (Documentation Map)

| 文档 | 作用 |
| --- | --- |
| `docs/harness/PROGRESS.md` | 当前进度、Next Steps、暂存任务（格式见文件内） |
| `docs/harness/DECISIONS.md` | 历史架构/业务决策（ADR 格式见文件内） |
| `docs/harness/ERRORS.md` | 错误索引；细节文件在 `docs/harness/errors/ERR-XXX.md` |
| `DEVELOPMENT_FLOW.md` | 四步法详细指南（含用户侧 Prompt 模板） |
| `产品方案.md` / `前端设计.md` / `README.md` | 产品愿景 / 前端设计 / API 参考 |

### 何时必须追加 DECISIONS.md

- ✅ 引入新的外部依赖；修改事件 schema 或数据模型（**事件流是持久化数据，schema 变更直接影响重放**）
- ✅ 修改公共 API 签名或删除已有接口；修改核心配置默认值；设计非预期的 workaround
- ❌ 纯内部重构（不改行为）、修复明显逻辑 bug、代码格式调整

## 3. 核心工作准则 (Working Style & Philosophy)

1. **谋定而后动**：写码前明确假设，不确定直接问；多种实现路径时列出选项与利弊，不默默选择；连续报错 >2 次停下分析日志或求助。
2. **极简主义**：只写解决当前问题的最少代码，不做"未来可能用到"的投机设计。
3. **外科手术式修改**：只动与任务直接相关的代码；绝不顺手重构、调整原有格式或注释；严格保持现有风格；只清理自己引入的孤儿代码。
4. **目标驱动执行**：把模糊任务转为可验证目标（"加校验" → "非法输入测试报错 → 修复 → 测试通过"），多步任务先输出带验证点的计划。
5. **调试代码规范**：Rust 统一用 `eprintln!("AGENT-DEBUG: {:?}", variable)`；前端统一用 `console.log("AGENT-DEBUG:", variable)`。清理时只删含 `AGENT-DEBUG` 的行，**严禁删除用户原有的调试代码**。
6. **安全红线**：❌ 删改 `.git/`、`.env`、secrets 类文件；❌ 破坏性系统命令；❌ 未经询问添加依赖；❌ 硬编码密码/token/API key；❌ 擅自格式化整个项目；❌ 运行交互式或挂起命令（`vim`、`tail -f` 等）。
7. **建议性约束**：不修改 `.gitignore` 既有规则；不新增 build/CI 配置（除非任务要求）；不直接修改 `DECISIONS.md` 中 `Status: Accepted` 的条目（应先提议）。
8. **下班自查清单**：无新增警告；新公共函数/类有 ≥1 行用途注释；改动文件数 ≤5（超了反思是否夹带重构）；PROGRESS.md 的 Next Steps 已更新；所有 Check 命令已通过。
