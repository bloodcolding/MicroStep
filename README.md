> **致谢**：本项目的产品思路参考了知乎回答《在明白什么道理后你就不再焦虑了？》—— 东沟红烧肉的回答，特此致谢。
> 原文链接：<https://www.zhihu.com/question/629658395/answer/2079668070579291301>

# MicroStep 2.0 · 个人成长 RPG

基于《产品方案.md》实现的本地优先、事件溯源个人成长系统。它不要求你每天打卡，而是通过手动 Task 记录客观事实，由系统结算属性数值变化、里程碑结项与称号加成。

技术形态：**Tauri v2 桌面应用**（Win / macOS / Linux），后端为 Rust 事件溯源内核，前端为原生 JS 单页应用（无构建步骤）。Python 版实现已由 `refactor-to-tauri-v2` 变更完成行为等价移植（golden replay 零差异验收），遗产代码保留至退役任务执行。

## 环境要求

- Rust 工具链（Windows：rustup + MSVC Build Tools + WebView2 运行时）
- 无需 Python、Node 或任何前端构建链（`npx @tauri-apps/cli` 仅在打安装包时按需使用）

## 运行与构建

```powershell
# 开发运行（调试构建，前端资源编译期内嵌）
cd src-tauri
cargo run

# 发布构建（产物：src-tauri\target\release\microstep.exe）
cargo build --release

# 安装包打包（NSIS 安装器，产物在 src-tauri\target\release\bundle\）
npx -y @tauri-apps/cli build
```

## 数据存储与迁移

事件流不再位于仓库工作目录，而是存储于系统应用数据目录（Tauri `app_data_dir()`）：

| 平台 | 数据目录 |
| --- | --- |
| Windows | `%APPDATA%\com.microstep.app\` |
| macOS | `~/Library/Application Support/com.microstep.app/` |
| Linux | `~/.local/share/com.microstep.app/` |

目录内 `events.jsonl` 是唯一持久化文件（append-only 事件流）；目录同时被初始化为 git 仓库骨架（不配远端、不自动提交），同步时写入快照 commit（见下节）。

### 远端同步（Git HTTPS + PAT）

数据目录可配置通用 git 远端（GitHub / Gitee / Gitea 等 HTTPS remote + PAT）实现多设备双向同步：

- **配置**：应用内「☁️ 远端同步」面板填写远端地址 / PAT（密码形态存储于数据目录 `sync.json`，建议使用细粒度单仓库读写 token 并定期轮换）/ 分支（默认 `main`）。远端信息只存 `sync.json`，不写入 `.git/config`——数据仓库由应用管理。
- **语义**：union merge（本地序保留 + 远端独有事件按序追加 + 按 `event_id` 语义相等去重）；同一 `event_id` 双端内容不同 = 违反事件不可变红线，整次拒绝并点名该 id，双端文件保持原样。
- **触发**：启动时异步 best-effort pull（失败仅记入最近结果，不阻塞不弹窗）；面板「立即同步」执行完整 pull-merge-push，返回拉取/推送/合并统计。同步全程持事件流锁（秒级），期间业务写入排队等待。
- **实现**：gix（fetch / 对象读写，纯 Rust）+ git2-rs（push，libgit2 进程内）双栈，零子进程；commit 仅在同步时写入（快照式历史），push 遇远端并发推进自动重跑（上限 2 次）。未配置或离线时全部本地功能不受影响。

**灾难恢复**：手工 `git clone` 远端仓库后，将其中 `events.jsonl` 拷贝至本机数据目录即可。

**从 Python 版迁移旧数据**（手动拷贝，单文件单用户）：

1. 关闭运行中的应用；
2. 将旧仓库内 `data/events.jsonl` 拷贝至上表数据目录；
3. 启动应用——已有事件文件**不会被覆盖、清空或重置**，应用直接重放该文件并继续追加，历史数据完整保留。

首次启动（数据目录为空）会自动创建默认里程碑「无限进步」。

## 已实现的核心机制

- **事件溯源**：所有用户行为都是 append-only JSONL 事件，状态由 `apply_event(State, Event) -> NewState` 重放得到。
- **事件标记删除**：删除事件只会追加 `deleted` 标记和删除时间，原始事件、Task 和属性状态都不会被删除或回滚。每个事件都记录 `name`、`changes`、`created_at`、`deleted`、`deleted_at`。
- **事件与 Task 解耦**：事件自带名称和数据变化字段，Task 只是基于事件流派生出的当前状态；删除或更新 Task 都不会改写原始事件。
- **多属性 Task**：用户自己创建 Task，可同时选择一个或多个属性，分别设置增益或减益数值，以及是否可重复记录。创建界面会显示每个属性的当前数值和变化后的预览。
  例如：早起 `体质 +5 / SAN +2`、阅读半小时 `知识 +10`、跑步 5 公里 `体质 +20`、攻克技术难点 `专业 +50`、熬夜 `专业 -10 / SAN -15`。
- **Task 属性范围**：Task 提供 SAN、体质、专业能力、知识、表达、良善、魅力；不再包含意志力。
- **SAN 保护**：如果 Task 的 SAN 扣减超过当前 SAN，前端会禁用完成按钮，后端也会拒绝该次结算。
- **Task 可更新和删除**：Task 列表支持编辑名称、归属、可重复状态和全部属性效果；已完成的一次性 Task 改为可重复后会自动恢复为可执行状态。删除 Task 会追加 `TASK_DELETED` 事件，历史事件保留。
- **可检索 Task 列表**：Task 列表支持按名称、属性、数值、里程碑、可重复状态进行关键词过滤；已完成的 Task 默认隐藏，可勾选「显示已完成」查看。
- **七项属性**：SAN、体质、专业能力、知识、表达、良善、魅力；取消 EXP 与等级体系。
- **六维雷达图**：雷达图展示体质、专业能力、知识、表达、良善、魅力，并显示完整属性名称。
- **SAN 日结**：SAN 每天从 100 开始，当天所有 Task 和事件在此基础上结算；跨零点由后台 Ticker 自动补每日结算。
- **里程碑列表**：里程碑与 Task 父子关联，支持关键词搜索；结项祭坛要求铭文。
- **里程碑可编辑**：支持修改里程碑名称、描述、主属性，以及对应称号的加成属性和百分比；已结项里程碑同样可以调整后续加成。
- **里程碑称号**：每个里程碑一一对应一个称号，创建时选择目标属性和加成百分比；结项后解锁，最多装备 3 个。
- **称号加成**：所有 Task 完成时的属性变化都会按已装备称号的对应属性百分比加成结算。
- **渐进式觉醒**：初见只显示 SAN，点击「立即觉醒」进入完整属性面板。
- **单实例保护**：同时只允许一个应用实例运行，防止并发写损坏事件流。

## 目录

```text
src-tauri/
  src/
    domain.rs       # 纯函数 Reducer、里程碑与称号（业务规则全部在此）
    store.rs        # EventStore：JSONL 事件流读写 + 业务命令（Mutex 保护，损坏行容错）
    app_state.rs    # AppState：14 个信封方法（强制转换语义收敛）
    commands.rs     # 14 个 #[tauri::command] IPC 薄封装
    data_dir.rs     # AppData 数据目录 + 手工 .git 骨架
    ticker.rs       # tokio 周期任务：跨零点补每日结算
  tests/            # domain/store/golden/commands/data_dir/ticker
frontend/           # 原生 JS 单页应用（index.html + js/ ES modules，无构建步骤）
openspec/           # 变更规格（refactor-to-tauri-v2 等）
docs/harness/       # 进度、决策（ADR）、错误索引
```

## IPC 命令参考

前端 `frontend/js/api.js` 是唯一 IPC 收口：`/api/...` 风格路径查表映射为 `invoke(命令, 参数)`（参数键自动转 camelCase），响应为与 HTTP 版逐字段等价的 `{ok, error, event, state}` 信封。

| 命令 | 对应原 HTTP | 说明 |
| --- | --- | --- |
| `get_state` | `GET /api/state` | 重放事件流并返回当前状态 |
| `get_meta` | `GET /api/meta` | 维度与称号定义、称号装备槽上限 |
| `create_epic` | `POST /api/epics` | 创建里程碑并生成对应称号 |
| `update_epic` | `POST /api/epics/{id}/update` | 更新里程碑和对应称号加成 |
| `complete_epic` | `POST /api/epics/{id}/complete` | 结项祭坛（铭文必填） |
| `create_task` | `POST /api/tasks` | 创建多属性 Task（`effects: [{dimension, delta}]`） |
| `log_task` | `POST /api/tasks/{id}/log`（`/complete` 别名） | 记录一次 Task，按多效果结算 |
| `update_task` | `POST /api/tasks/{id}/update` | 更新 Task 名称、归属、可重复、效果 |
| `delete_task` | `POST /api/tasks/{id}/delete` | 标记删除 Task，历史事件保留 |
| `delete_event` | `POST /api/events/{id}/delete` | 给事件追加 `deleted` 标记 |
| `equip_title` | `POST /api/titles/equip` | 装备已解锁称号（最多 3 个） |
| `unequip_title` | `POST /api/titles/unequip` | 卸下称号 |
| `awaken` | `POST /api/awaken` | 提前觉醒六维雷达 |
| `system_tick` | `POST /api/system/tick` | 手动触发每日结算检查 |
| `sync_get_config` | —（新增） | 读同步配置（PAT 脱敏，仅末 4 位可辨识） |
| `sync_set_config` | —（新增） | 部分更新同步配置（未携带字段不变；`pat: ""` 清除） |
| `sync_now` | —（新增） | 完整 pull-merge-push，成功返回 `{pulled, pushed, merged}` |

创建 Task 的参数示例：

```json
{
  "title": "跑步后学习",
  "epic_id": "",
  "repeatable": false,
  "effects": [
    { "dimension": "physical", "delta": 20 },
    { "dimension": "knowledge", "delta": 10 },
    { "dimension": "san", "delta": -15 }
  ]
}
```

维度白名单：`san, physical, professional, knowledge, expression, kindness, charm`。不存在 willpower / EXP / 等级体系。完整规格见 `openspec/changes/refactor-to-tauri-v2/specs/`。

## 测试

```powershell
cd src-tauri
cargo test    # 79 例：domain 31 + store 18 + golden 3 + commands 22 + data_dir 3 + ticker 2
```

golden 回归：以 Python 版导出的 State 快照（真实流 + 构造序列覆盖全部 14 种事件）为基准资产，Rust 版重放相同输入要求归一化零差异。

## 数据主权

事件流保存在本机 AppData 数据目录，修改 `src-tauri/src/domain.rs` 的 Reducer 后重放同一事件流即可升级规则；历史事件永远 append-only 保留。数据目录已具备 git 仓库结构，远端同步（GitHub/Gitee 等）为规划的下一个变更。

## 产品与设计文档

- 产品愿景：`产品方案.md`
- 前端交互：`前端设计.md`
- 进度与决策：`docs/harness/PROGRESS.md`、`docs/harness/DECISIONS.md`
