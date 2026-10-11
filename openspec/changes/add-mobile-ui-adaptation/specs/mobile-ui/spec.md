# mobile-ui · 规格增量

## ADDED Requirements

### Requirement: 移动视口与安全区

前端 SHALL 声明支持移动 WebView 的响应式视口（`width=device-width`、`initial-scale=1.0`、`viewport-fit=cover`），并在移动布局中通过系统 safe area 环境值预留顶部与底部遮挡区。safe area 环境值缺失时 SHALL 以 0 缺省并保持内容可用。根页面 SHALL 抑制整页橡皮筋回弹，同时保留用户缩放能力。

#### Scenario: 异形屏安全区

- **WHEN** 在带顶部刘海 / 灵动岛 inset 的移动设备上打开应用
- **THEN** 顶栏与页面内容不被系统遮挡区覆盖

#### Scenario: 底部手势条避让

- **WHEN** 移动设备存在非零底部 safe area inset
- **THEN** 页面底部内容与主要操作不被 Home 指示条 / 手势条遮挡

#### Scenario: 无 inset 设备回退

- **WHEN** 目标 WebView 未暴露 safe area inset
- **THEN** 布局按 0 inset 回退，不出现空白异常或功能不可用

#### Scenario: 保持用户缩放能力

- **WHEN** 审计 `index.html` viewport 声明
- **THEN** 未使用禁止用户缩放的 `user-scalable=no`

### Requirement: 小屏响应式布局

应用 SHALL 在手机宽度断点内提供单列移动布局：顶栏紧凑排列、卡片与表单占满可用宽度、Dashboard 左右栏折叠为纵向顺序、页面主内容无横向滚动。手机横屏 SHALL 保持同一信息结构并压缩纵向占用。960px 及以上桌面布局 SHALL 保持既有双栏结构与窗口约束。

#### Scenario: 初见期竖屏

- **WHEN** 以约 390×844 CSS 像素查看 simpleView
- **THEN** hero 卡、SAN 条、新建 Task 与最近事件流垂直排布，无横向滚动

#### Scenario: 仪表盘竖屏

- **WHEN** 以约 390×844 CSS 像素查看 dashboardView
- **THEN** 里程碑、Task、事件流、SAN、雷达图、属性卡与称号区折叠为可纵向浏览的单列顺序

#### Scenario: 手机横屏

- **WHEN** 以约 740×360 CSS 像素查看任一视图
- **THEN** 顶栏与卡片高度被压缩，主要操作仍可见且无需横向滚动

#### Scenario: 桌面布局不回归

- **WHEN** 以 1280×860 或 960×640 CSS 像素查看应用
- **THEN** 既有桌面双栏布局、顶栏操作与卡片密度不套用小屏紧凑规则

### Requirement: 触控与输入可用性

移动布局中的主要交互控件 SHALL 具备不小于 44×44 CSS 像素的可命中区域；视觉尺寸较小时 SHALL 通过 padding、伪元素或等效方式扩展命中区。交互状态 SHALL 不依赖 hover，并 SHALL 提供可感知的 active / focus-visible 反馈。文本输入、下拉与多行文本控件 SHALL 保持可选中、可复制、可聚焦，移动字号 SHALL 避免聚焦时强制放大。

#### Scenario: 触控目标尺寸

- **WHEN** 在移动断点检查顶栏按钮、任务完成 / 编辑 / 删除、称号装备与模态操作控件
- **THEN** 每个主要可点击控件的命中区域至少为 44×44 CSS 像素

#### Scenario: 非依赖 hover

- **WHEN** 在无 hover 的触控设备上操作全部按钮
- **THEN** 可用性不依赖 hover 状态，active / focus-visible 有可感知反馈

#### Scenario: 输入框行为

- **WHEN** 在移动端聚焦 Task / 里程碑 / 同步表单输入控件
- **THEN** 输入内容可选中复制，控件字号不触发异常自动放大

#### Scenario: 按钮双击不缩放

- **WHEN** 用户快速连续点击主操作按钮
- **THEN** 页面不因按钮双击触发缩放

### Requirement: 移动模态与返回栈

前端 SHALL 为打开中的覆盖层维护标准 History API 返回栈。移动端系统返回 / 手势返回 SHALL 先关闭当前最上层模态覆盖层；无覆盖层时 SHALL NOT 拦截宿主应用的默认退出行为。小屏模态 SHALL 以 bottom sheet 或近全屏形态展示并预留安全区；桌面模态 SHALL 保持既有居中形态。所有关闭路径关闭后 SHALL NOT 遗留会导致无效后退的重复历史状态。

#### Scenario: Android 返回关闭设置面板

- **WHEN** 在 Android 上打开设置面板后触发系统返回
- **THEN** 设置面板关闭并回到下层视图，应用不退出

#### Scenario: Android 返回关闭业务表单

- **WHEN** 打开新建 Task、编辑里程碑或结项祭坛后触发系统返回
- **THEN** 当前模态关闭且未提交内容随既有关闭语义丢弃，应用不退出

#### Scenario: 常规关闭路径

- **WHEN** 通过 ×、取消按钮或点击遮罩关闭模态
- **THEN** 模态关闭且后续系统返回不产生已关闭模态的幽灵返回

#### Scenario: 无覆盖层时不拦截退出

- **WHEN** 没有任何前端覆盖层打开且用户触发系统返回
- **THEN** 前端不消费返回事件，退出 / 后退行为交还宿主应用

#### Scenario: 桌面模态形态保留

- **WHEN** 在桌面宽度打开任一模态
- **THEN** 模态保持既有居中卡片形态与关闭方式

### Requirement: 软键盘避让

移动端软键盘弹出时，前端 SHALL 通过 `visualViewport` 或等效标准机制调整覆盖层可用高度，并保证聚焦输入可滚入可见区。键盘弹出期间用户已输入内容 SHALL NOT 丢失，提交 / 取消路径 SHALL 保持可达或可滚动到达。`visualViewport` 不可用时 SHALL 回退到既有 viewport / focus 滚动行为且不导致脚本错误。

#### Scenario: 聚焦底部输入

- **WHEN** 在移动端打开 Task 表单并聚焦靠下方的输入控件
- **THEN** 软键盘弹出后聚焦控件仍可见，未被键盘永久遮挡

#### Scenario: 键盘打开时操作可达

- **WHEN** 软键盘保持打开状态
- **THEN** 用户可以滚动到达提交或取消操作，输入内容不丢失

#### Scenario: 不支持 visualViewport 的回退

- **WHEN** WebView 不提供 `visualViewport` API
- **THEN** 模态仍可打开、滚动和提交，不产生脚本错误

### Requirement: 雷达图自适应

属性雷达图 SHALL 按其 CSS 容器尺寸与 `devicePixelRatio` 自适应绘制，并在容器尺寸或设备方向变化时重绘。移动断点下维度标签 SHALL 保持可读且不被裁剪。

#### Scenario: 小屏清晰绘制

- **WHEN** 以约 390×844 CSS 像素和 devicePixelRatio 3 查看仪表盘
- **THEN** 雷达图按容器与物理像素比例清晰绘制，六维标签完整可见

#### Scenario: 旋转屏幕

- **WHEN** 移动设备从竖屏旋转到横屏
- **THEN** 雷达图按新容器尺寸重绘，无拉伸、模糊或标签裁剪

### Requirement: 移动适配工程边界

移动 UI 实现 SHALL 保持前端零构建与零 npm 依赖，且 SHALL NOT 修改 IPC command、Rust 业务内核、事件 schema、同步配置、Tauri plugin、capabilities 或 iOS / Android 原生权限声明。新增用户可见文案与 aria-label SHALL 进入既有 i18n 字典并覆盖 `zh-CN` / `en`。

#### Scenario: 零构建零依赖审计

- **WHEN** 审计本变更后的 `frontend/` 与依赖清单
- **THEN** 无 package.json / node_modules / 打包器配置 / npm 依赖，新增模块为原生 ES module

#### Scenario: 平台与数据边界审计

- **WHEN** 对比本变更前后的 Rust、IPC、事件 schema、`Cargo.lock`、capabilities 与原生工程权限文件
- **THEN** 上述文件无本变更引入的行为性修改

#### Scenario: 新增文案本地化

- **WHEN** 移动适配引入新的用户可见文案或 aria-label
- **THEN** `zh-CN` 与 `en` 字典均提供对应 key，运行时无 i18n 缺失警告
