# settings-ui Specification

## Purpose
TBD - created by archiving change add-settings-panel. Update Purpose after archive.
## Requirements
### Requirement: 设置入口

应用顶栏 SHALL 提供专门的「设置」按钮，且在 simpleView（初见期）与 dashboardView（觉醒后）两种视图状态下均 SHALL 可见可用；点击 SHALL 打开模态设置面板，面板 SHALL 复用既有统一模态容器的打开与关闭交互（右上 × / 点击遮罩 / 取消按钮）。

#### Scenario: 初见期可达

- **WHEN** 用户尚未觉醒（simpleView 展示）时点击顶栏「设置」
- **THEN** 设置面板打开并可见全部配置分类（代码仓同步在觉醒前即可管理）

#### Scenario: 觉醒后可达

- **WHEN** 用户已觉醒（dashboardView 展示）时点击顶栏「设置」
- **THEN** 设置面板同样打开，行为与初见期一致

#### Scenario: 关闭交互一致

- **WHEN** 用户通过 × / 遮罩 / 取消任一方式关闭设置面板
- **THEN** 面板关闭且不残留 DOM 状态，主界面数据不受影响

### Requirement: 分类切换

设置面板 SHALL 提供恰好三个配置分类：代码仓同步、数据档案、关于。用户 SHALL 能点击分类标签在三者间切换，切换 SHALL NOT 触发任何网络请求（纯本地显隐）。打开面板 SHALL 默认定位「代码仓同步」分类，并刷新一次该分类的配置回显。

#### Scenario: 三分类齐全

- **WHEN** 打开设置面板
- **THEN** 可见代码仓同步、数据档案、关于三个分类入口，无多余分类

#### Scenario: 切换零请求

- **WHEN** 在三个分类间来回切换
- **THEN** 不发出任何 IPC 请求（打开面板时的单次配置刷新除外）

#### Scenario: 默认分类

- **WHEN** 打开设置面板
- **THEN** 默认展示「代码仓同步」分类，且远端地址 / PAT 脱敏状态 / 分支 / 最近同步结果已按当前配置回显

### Requirement: 代码仓同步分类

「代码仓同步」分类 SHALL 完整承接既有同步配置界面与行为语义：远端 URL / PAT（密码形态）/ 分支表单、PAT 留空保存保持不变、勾选「清除已保存 PAT」发送空串、「保存配置」调 `sync_set_config`、「立即同步」前自动保存表单（保存失败中止同步）且调 `sync_now`、最近同步时间与结果展示、成功后刷新状态。调用链 SHALL 经 `api.js` 唯一收口，IPC command 契约与数量 SHALL 零变化。

#### Scenario: 表单等价迁移

- **WHEN** 在设置面板「代码仓同步」分类填写远端信息保存后点击「立即同步」
- **THEN** 行为与迁移前的同步卡片一致：先自动保存（失败则中止并 toast），再执行同步并展示 pulled/pushed/merged 统计或分类错误文案

#### Scenario: PAT 处理语义不变

- **WHEN** PAT 输入框留空保存 / 勾选清除保存 / 输入新 PAT 保存
- **THEN** 分别保持既有 PAT、清除既有 PAT、更新为输入值，回显仍为脱敏形态

#### Scenario: IPC 零变化

- **WHEN** 审计设置面板的调用链
- **THEN** 全部经 `api.js` 既有路由收口，无新增或修改的 IPC command（总数仍 17）

### Requirement: 数据档案分类

「数据档案」分类 SHALL 只读展示：数据落点说明（本地 AppData 数据目录口径）、事件总数（取自已加载的应用状态）、最近同步时间与结果。该分类 SHALL NOT 提供任何数据修改或删除入口。

#### Scenario: 只读展示

- **WHEN** 切换到「数据档案」分类
- **THEN** 可见数据落点说明、事件总数与最近同步信息，且不存在任何修改/删除控件

### Requirement: 关于分类

「关于」分类 SHALL 以静态文案展示应用名称、产品定位与事件溯源机制的一句话说明，SHALL NOT 发起任何请求。

#### Scenario: 静态展示

- **WHEN** 切换到「关于」分类
- **THEN** 展示静态说明文案，无网络请求、无动态数据依赖

### Requirement: 唯一配置入口

仪表盘右栏 SHALL NOT 再保留远端同步卡片或同步表单；代码仓同步的配置入口 SHALL 唯一化为设置面板的「代码仓同步」分类。

#### Scenario: 旧卡移除

- **WHEN** 觉醒后查看 dashboardView 右栏
- **THEN** 不存在远端同步卡片，同步相关操作只能在设置面板完成

### Requirement: 工程约束

settings-ui 实现 SHALL 保持前端原生 ES modules、无构建步骤、零 npm 依赖；前端模块改动 SHALL 收敛为：新增 `settings.js` 一个模块 + 改造 `sync.js` + `index.html` 挂载调整 + `styles.css` 样式补充（连同 `前端设计.md` 文档更新，改动 ≤5 文件）；面板内容 SHALL 使用 div 结构而非 `<form>`（规避既有 modalBody 统一 submit 代理的无关命中）；所有动态文案进模板前 SHALL `escapeHtml`。

#### Scenario: 依赖与构建审计

- **WHEN** 审计 `frontend/` 目录与模块引入
- **THEN** 无 package.json / node_modules / 构建产物，模块为原生 ES modules 被 index.html 直接引入

#### Scenario: 改动收敛

- **WHEN** 对比变更前后仓库 diff
- **THEN** 前端改动仅涉及 settings.js（新增）/ sync.js / index.html / styles.css 与前端设计文档，无 Rust 源码改动
