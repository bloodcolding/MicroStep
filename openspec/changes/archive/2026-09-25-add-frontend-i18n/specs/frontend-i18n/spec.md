# frontend-i18n · 规格增量

## ADDED Requirements

### Requirement: 运行时 i18n 字典

前端 SHALL 内置运行时 i18n 模块，以原生 ES module 字典提供 `zh-CN` 与 `en` 两套界面文案，并暴露统一翻译函数 `t(key, params)`；SHALL NOT 引入构建步骤、打包器或 npm 依赖。当当前语言字典缺失某 key 时，`t()` SHALL 回退 `zh-CN` 字典值并输出一次含该 key 的 `console.warn`。含计数语义的文案 SHALL 支持简单复数：字典提供 `.one` / `.other` 变体，`t()` SHALL 依 `Intl.PluralRules` 按参数中的 count 值选择变体（无 count 时用 `.other`）。新增语言 SHALL 仅需新增字典文件并注册，SHALL NOT 要求改动既有调用方。

#### Scenario: 翻译命中与插值

- **WHEN** 当前语言为 `en` 且 key 存在于 en 字典
- **THEN** `t()` 返回 en 文案，`params` 插值占位符被正确替换

#### Scenario: 缺失回退

- **WHEN** 当前语言为 `en` 且某 key 仅存在于 zh-CN 字典
- **THEN** 界面展示 zh-CN 文案，且控制台输出含该 key 的 `console.warn`

#### Scenario: 零构建审计

- **WHEN** 审计 `frontend/` 目录
- **THEN** 无 package.json / node_modules / 打包器配置，i18n 模块为原生 ES modules 且被既有模块直接 import

#### Scenario: 计数文案复数形态

- **WHEN** 当前语言为 `en` 且以 count = 1 与 count = 2 渲染任务计数文案
- **THEN** 分别展示单数（如 `1 Task`）与复数（如 `2 Tasks`）形态；`zh-CN` 下同一文案无形态差异

### Requirement: 语言选择与持久化

应用首次启动 SHALL 依据 `navigator.language` 判定默认语言（`zh` 前缀语言 → `zh-CN`，否则 `en`）；用户手动切换语言后 SHALL 将所选语言持久化于 `localStorage`；后续启动 SHALL 以 localStorage 值优先。localStorage 值非法或非受支持语言时 SHALL 回退系统语言判定规则，SHALL NOT 导致启动失败。语言偏好 SHALL NOT 写入事件流或同步配置文件。

#### Scenario: 首启跟随系统

- **WHEN** 无 localStorage 记录且系统语言为 `zh-TW`
- **THEN** 默认语言为 `zh-CN`；系统语言为 `en-US` 时默认 `en`

#### Scenario: 手动切换持久化

- **WHEN** 用户切换到 `en` 后重启应用
- **THEN** 界面仍为 `en`，即使系统语言为中文

#### Scenario: 非法存储值容错

- **WHEN** localStorage 中语言值为非受支持语言（如 `fr`）
- **THEN** 按系统语言判定规则回退，应用正常启动

#### Scenario: 档案零污染

- **WHEN** 用户切换语言
- **THEN** `events.jsonl` 与 `sync.json` 不产生任何写入

### Requirement: 顶栏语言切换入口

应用顶栏 SHALL 提供语言切换控件，且在 simpleView（初见期）与 dashboardView（觉醒后）两种视图状态下均 SHALL 可见可用；点击 SHALL 即时切换语言并刷新全部可见文案；切换 SHALL NOT 发起任何 IPC 请求。

#### Scenario: 两视图可达

- **WHEN** 分别在初见期与觉醒后状态查看顶栏
- **THEN** 语言切换控件均可见可用，行为一致

#### Scenario: 即时刷新

- **WHEN** 用户在任一视图点击切换语言
- **THEN** 顶栏、卡片、列表、提示等全部可见文案更新为目标语言，无混合语言残留

#### Scenario: 零 IPC

- **WHEN** 审计语言切换动作的调用链
- **THEN** 不产生任何 IPC 请求，语言状态变更纯本地完成

### Requirement: 静态文案本地化

`index.html` 中的静态用户可见文案（含页面 title、按钮与标题文本、输入框 placeholder、aria-label）SHALL 以声明式标记（`data-i18n` 系列）声明其文案 key，由 i18n 模块在启动与语言切换时批量应用；`<html lang>` 属性 SHALL 与当前语言同步。

#### Scenario: 启动即本地化

- **WHEN** 应用以语言 `en` 启动完成
- **THEN** index.html 全部静态文案（含 title、placeholder、aria-label）为 en，`<html lang>` 为 `en`

#### Scenario: 切换后静态刷新

- **WHEN** 用户从 `en` 切换到 `zh-CN`
- **THEN** 静态文案连同 placeholder 与 aria-label 一并更新，`<html lang>` 同步为 `zh-CN`

### Requirement: 动态文案本地化

前端 JS 生成的用户可见文案（toast 提示、表单校验提示、事件流描述、SAN 水位提示、空态与计数、设置面板与同步面板文案等）SHALL 经 `t()` 输出；含变量的文案 SHALL 使用字典插值占位符而非硬编码拼接。

#### Scenario: 事件流描述本地化

- **WHEN** 以 `en` 查看最近事件流
- **THEN** 前端生成的事件类型描述为 en 文案

#### Scenario: 提示与校验本地化

- **WHEN** 以 `en` 触发保存失败 toast 或表单校验提示
- **THEN** 前端生成的提示文案为 en，插值变量（如数量、名称占位）位置正确

### Requirement: 维度名与数字日期本地化

7 个白名单维度（san / physical / professional / knowledge / expression / kindness / charm）的显示名 SHALL 由前端字典按当前语言提供；字典未覆盖的维度 key SHALL 回退后端 meta 下发名，再回退 key 原文。数字与日期时间格式化 SHALL 使用与当前语言对应的 locale（`zh-CN` → `zh-CN`，`en` → `en-US`）。

#### Scenario: 维度名语言切换

- **WHEN** 以 `en` 查看属性雷达与属性值卡片
- **THEN** 7 个维度显示名为 en 字典值

#### Scenario: 未知维度兜底

- **WHEN** 后端 meta 出现字典未覆盖的维度 key
- **THEN** 显示 meta 下发名；meta 亦缺失时显示 key 原文

#### Scenario: 格式化跟随

- **WHEN** 语言从 `zh-CN` 切换到 `en`
- **THEN** 数值（维度值 / 预览值）与日期时间展示格式随 locale 切换

### Requirement: 翻译边界

用户数据（Task / 里程碑的标题、描述、铭文、备注等用户输入内容）与后端返回的错误消息原文 SHALL NOT 被翻译、改写或二次包装；界面 SHALL 直接呈现其原文。

#### Scenario: 用户数据保持原文

- **WHEN** 以 `en` 查看含中文标题的 Task / 里程碑
- **THEN** 标题、描述、铭文等用户输入内容原样显示

#### Scenario: 后端错误原文直显

- **WHEN** 后端返回中文错误消息（如同步认证失败文案）且当前语言为 `en`
- **THEN** 该错误消息原文直接展示，不经过前端字典翻译
