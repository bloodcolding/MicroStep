# AGENTS.md · MicroStep 2.0

本仓库是 **Python 3.12+ 纯标准库实现的事件溯源个人成长 RPG**（本地优先，`pyproject.toml` 要求 ≥3.12，零第三方依赖，无需 `pip install`）。前端为原生 JS 单页应用，无构建步骤。产品愿景见 `产品方案.md`，API 全表见 `README.md`。

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
| 启动服务（默认 <http://127.0.0.1:8765>） | `python run.py` |
| 隔离数据启动 | `python run.py --port 8768 --data data\test_events.jsonl` |
| 全量测试 = 唯一 Check 命令 | `python tests\smoke.py` |

- `python tests\smoke.py` 使用独立的 `data/smoke_events.jsonl`（运行前后自动清理），**不会污染真实事件流**，可随时放心运行。
- 本仓库**没有** lint / typecheck / formatter / 构建配置，不要自行引入；"检查通过" = smoke 测试通过。
- 启动服务器是挂起命令：必须后台运行或提示用户手动执行，禁止阻塞主对话终端。
- 前端改动刷新浏览器即生效；后端改动需重启服务器（无热重载）。

### 架构与硬性不变量

```text
backend/domain.py   纯函数 Reducer（applyEvent/build_state）+ 维度/称号定义 —— 全部业务规则在此
backend/store.py    EventStore：JSONL 事件流读写 + 业务命令（RLock 保护，损坏行容错跳过）
backend/server.py   标准库 ThreadingHTTPServer：/api/* 路由 + 托管 frontend/ 静态文件 + 后台 Ticker 线程自动补每日结算
frontend/           原生 JS 单页应用（index.html/app.js/styles.css），无构建步骤
data/events.jsonl   唯一持久化：append-only 事件流（真实用户数据，已纳入 Git）
```

- **零第三方依赖是设计原则**：`pyproject.toml` 的 `dependencies = []`。引入任何新依赖（含传递依赖）必须先征得用户同意并追加 ADR。
- **事件不可变**：绝不改写或删除 `data/events.jsonl` 中的行；"删除" = 追加 `deleted` 标记事件（tombstone）；Task 只是事件流的投影，删改 Task 不回写历史事件。修改业务规则 = 修改 `domain.py` 的 Reducer，重放同一事件流即完成规则升级。
- **真实数据保护**：`data/events.jsonl` 是用户真实数据。任何实验必须用 `--data` 指定独立文件，或设置环境变量 `MICROSTEP_EVENT_PATH` 覆盖路径。
- **维度白名单**：`san, physical, professional, knowledge, expression, kindness, charm`。**不存在** willpower / EXP / 等级体系 —— smoke 测试断言其不存在，禁止重新引入。
- **数值规则**：SAN 是日槽（clamp 0–100，每日从 100 重置，扣减超过当前 SAN 时后端拒绝结算）；其余六维为 pool（clamp ≥ 0）。称号加成按 `1 + title_bonus_percent / 100` 结算，最多装备 3 个。
- API 路由与请求体示例见 `README.md`；产品口径见 `产品方案.md`；前端交互设计见 `前端设计.md`。

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
5. **调试代码规范**：Python 统一用 `print("// AGENT-DEBUG:", variable)`；清理时只删含 `// AGENT-DEBUG` 的行，**严禁删除用户原有的调试代码**。
6. **安全红线**：❌ 删改 `.git/`、`.env`、secrets 类文件；❌ 破坏性系统命令；❌ 未经询问添加依赖；❌ 硬编码密码/token/API key；❌ 擅自格式化整个项目；❌ 运行交互式或挂起命令（`vim`、`tail -f` 等）。
7. **建议性约束**：不修改 `.gitignore` 既有规则；不新增 build/CI 配置（除非任务要求）；不直接修改 `DECISIONS.md` 中 `Status: Accepted` 的条目（应先提议）。
8. **下班自查清单**：无新增警告；新公共函数/类有 ≥1 行用途注释；改动文件数 ≤5（超了反思是否夹带重构）；PROGRESS.md 的 Next Steps 已更新；所有 Check 命令已通过。
