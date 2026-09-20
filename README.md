# MicroStep 2.0 · 个人成长 RPG

基于《产品方案.md》实现的本地优先、事件溯源个人成长系统。它不要求你每天打卡，而是通过手动 Task 记录客观事实，由系统结算属性数值变化、里程碑结项与称号加成。

## 运行

环境要求：Python 3.10+，无需安装第三方依赖。

```powershell
python run.py
```

浏览器打开 <http://127.0.0.1:8765>。

**一键启动（Windows）**：双击项目根目录的 `start.bat`，自动启动服务并在就绪后打开浏览器；关闭该窗口或按 Ctrl+C 即停止服务。

首次启动会在 `data/events.jsonl` 写入不可变事件流，并创建默认里程碑「无限进步」。

需要隔离测试数据时可以指定独立事件流：

```powershell
python run.py --port 8768 --data data\test_events.jsonl
```

## 已实现的核心机制

- **事件溯源**：所有用户行为都是 append-only JSONL 事件，状态由 `applyEvent(State, Event) -> NewState` 重放得到。
- **事件标记删除**：删除事件只会追加 `deleted` 标记和删除时间，原始事件、Task 和属性状态都不会被删除或回滚。每个事件都记录 `name`、`changes`、`created_at`、`deleted`、`deleted_at`。
- **事件与 Task 解耦**：事件自带名称和数据变化字段，Task 只是基于事件流派生出的当前状态；删除或更新 Task 都不会改写原始事件。
- **多属性 Task**：用户自己创建 Task，可同时选择一个或多个属性，分别设置增益或减益数值，以及是否可重复记录。创建界面会显示每个属性的当前数值和变化后的预览。
  例如：早起 `体质 +5 / SAN +2`、阅读半小时 `知识 +10`、跑步 5 公里 `体质 +20`、攻克技术难点 `专业 +50`、熬夜 `专业 -10 / SAN -15`。
- **Task 属性范围**：Task 提供 SAN、体质、专业能力、知识、表达、良善、魅力；不再包含意志力。
- **SAN 保护**：如果 Task 的 SAN 扣减超过当前 SAN，前端会禁用完成按钮，后端也会拒绝该次结算。
- **Task 可更新和删除**：Task 列表支持编辑名称、归属、可重复状态和全部属性效果；已完成的一次性 Task 改为可重复后会自动恢复为可执行状态。删除 Task 会追加 `TASK_DELETED` 事件，历史事件保留。
- **无解析入口**：系统不再从自然语言推断属性或数值，所有新增 Task 都由用户手动定义。
- **可检索 Task 列表**：Task 列表支持按名称、属性、数值、里程碑、可重复状态进行关键词过滤，支持纵向滚动；已完成的 Task 默认隐藏，可勾选「显示已完成」查看。
- **七项属性**：SAN、体质、专业能力、知识、表达、良善、魅力；取消 EXP 与等级体系。
- **六维雷达图**：雷达图展示体质、专业能力、知识、表达、良善、魅力，并显示完整属性名称。
- **SAN 日结**：SAN 每天从 100 开始，当天所有 Task 和事件在此基础上结算；进入下一天时记录前一天最终 SAN，然后重置为 100。
- **里程碑列表**：里程碑与 Task 父子关联，支持关键词搜索和纵向滚动；结项祭坛要求铭文。
- **里程碑可编辑**：支持修改里程碑名称、描述、主属性，以及对应称号的加成属性和百分比；已结项里程碑同样可以调整后续加成。
- **里程碑称号**：每个里程碑一一对应一个称号，创建时选择目标属性和加成百分比；结项后解锁，最多装备 3 个。
- **称号加成**：所有 Task 完成时的属性变化都会按已装备称号的对应属性百分比加成结算。
- **渐进式觉醒**：初见只显示 SAN，点击「立即觉醒」进入完整属性面板。

## 目录

```text
backend/
  domain.py     # 纯函数 Reducer、里程碑与称号
  store.py      # JSONL 事件流存储与业务命令
  server.py     # 标准库 HTTP 服务
frontend/
  index.html    # 单页应用
  styles.css
  app.js
tests/
  smoke.py      # 核心链路烟测
run.py
```

## 常用 API

| 方法 | 路径 | 说明 |
| --- | --- | --- |
| GET | `/api/state` | 重放事件流并返回当前状态 |
| GET | `/api/meta` | 返回维度与称号定义、称号装备槽上限（`max_equipped_titles`） |
| POST | `/api/epics` | 创建里程碑并生成对应称号（`title_bonus_dimension`、`title_bonus_percent`） |
| POST | `/api/epics/{id}/update` | 更新里程碑和对应称号加成 |
| POST | `/api/epics/{id}/complete` | 结项祭坛 |
| POST | `/api/tasks` | 创建多属性 Task（`effects: [{dimension, delta}]`、`repeatable`、可选 `epic_id`） |
| POST | `/api/tasks/{id}/log` | 记录一次 Task，按设置的多个效果结算 |
| POST | `/api/tasks/{id}/complete` | 兼容旧接口，等价于记录一次 Task |
| POST | `/api/tasks/{id}/update` | 更新 Task 名称、归属、可重复状态和属性效果 |
| POST | `/api/tasks/{id}/delete` | 标记删除 Task，保留全部历史事件 |
| POST | `/api/titles/equip` | 装备已解锁的里程碑称号，最多 3 个 |
| POST | `/api/titles/unequip` | 卸下称号 |
| POST | `/api/awaken` | 提前觉醒八维雷达 |
| POST | `/api/system/tick` | 手动注入每日 Tick |
| POST | `/api/events/{event_id}/delete` | 给事件追加 `deleted` 标记和删除时间，不删除原始事件或 Task |

创建多属性 Task 的请求体示例：

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

创建里程碑并生成对应称号：

```json
{
  "title": "上线企业级知识检索系统",
  "description": "完成检索系统的核心交付",
  "main_dimension": "professional",
  "title_bonus_dimension": "professional",
  "title_bonus_percent": 10,
  "title_emoji": "🏛️"
}
```

里程碑结项后对应称号解锁；装备后，所有 Task 中 `title_bonus_dimension` 对应的属性变化按 `1 + title_bonus_percent / 100` 结算。

## 测试

```powershell
python tests\smoke.py
```

## 数据主权

事件流保存在 `data/events.jsonl`，可直接纳入 Git。单条事件可以在前端事件流中标记删除，原始事件会一直保留。修改 `backend/domain.py` 的 Reducer 后重放同一事件流即可升级规则。
