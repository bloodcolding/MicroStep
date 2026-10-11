# design.md · add-mobile-ui-adaptation

## Context

MicroStep 前端是内嵌于 Tauri v2 的原生 JS 单页应用：两个互斥视图（初见期 / 觉醒仪表盘）+ 一个统一 modal 容器 + toast。当前 CSS 已有 1040px / 680px 断点雏形，但整体信息密度、按钮命中区、列表高度、模态框与软键盘行为仍按桌面设计；移动包虽可运行，未形成 Native App 的操作感。

硬约束来自仓库架构与既有规格：前端零构建、零 npm 依赖；`api.js` 是唯一 IPC 收口；Rust 内核、事件 schema、IPC command 与移动原生权限均不动。

## Goals / Non-Goals

**Goals:**

- 让 Android / iOS WebView 在常见手机竖屏与横屏尺寸下无横向滚动、无安全区遮挡、可完成全部既有操作。
- 让触控目标、返回手势、模态框和软键盘行为符合移动应用习惯。
- 保持 960px 以上桌面视觉与交互不回归。
- 以标准 CSS / Web API 实现，继续单一 `index.html` 入口。

**Non-Goals:**

- 不引入 Konsta / Ionic / Framework7 / Vant / Tailwind 或任何 npm 包。
- 不新增 `desktop.html` / `mobile.html` 双入口或构建流程。
- 不实现多页面路由；simpleView / dashboardView 仍由 `profile.awakened` 驱动。
- 不新增相机、文件、通知、生物识别等系统能力或原生权限。
- 不改变 IPC、Rust Reducer、事件流 schema、同步配置与数据档案。

## Decisions

### D1 · 单入口响应式融合，而非移动端独立 UI

| | 单入口 + media query / coarse pointer 增强 ✅ | 双入口分离 | 引入移动 UI 框架 |
|---|---|---|---|
| 成本 | 中，主要改 CSS 与少量 overlay 逻辑 | 高，需构建入口与状态复用层 | 高，破坏零依赖 |
| 一致性 | 两端共享 i18n、渲染与状态 | 需长期同步两套视图 | 依赖框架组件语义 |
| 风险 | 小屏布局需真机验证 | 远超当前需求 | 违反 `app-shell` 规格 |

当前应用只有一个业务页面与少量浮层，不存在桌面多窗口 / 右键菜单等强分叉交互。响应式融合足以覆盖需求，且不会破坏既有 17 个渲染 / 状态模块的工程约定。

### D2 · 视口、safe area 与滚动模型

- `index.html` viewport 使用 `width=device-width, initial-scale=1.0, viewport-fit=cover`。
- 不使用 `user-scalable=no` 剥夺用户缩放能力；交互控件用 `touch-action: manipulation` 消除按钮双击缩放，同时保留页面可访问性。
- 以 `env(safe-area-inset-top)` / `env(safe-area-inset-bottom)` 为 app 外壳预留系统遮挡区，缺省值为 0，兼容 Android inset 不暴露的环境。
- 根页面承担主滚动，禁用整页橡皮筋回弹；Task / 里程碑 / 事件等既有内部列表在小屏减少固定 `max-height`，优先自然展开，避免“滚动套滚动”。

### D3 · 断点与布局策略

- 保留既有 1040px 断点：Dashboard / hero 在窄屏折叠单列。
- 新增移动断点（约 760px）：顶栏压缩、卡片内边距收缩、操作按钮换行或等宽排布、Task / 事件行允许次要信息下沉。
- 对手机横屏（如宽度仍处移动断点但高度小于约 480px）仅压缩顶栏与卡片间距，不改变信息结构。
- 960px 及以上不套用移动紧凑规则，桌面双栏与现有窗口 `minWidth: 960` 保持不变。

### D4 · 覆盖层历史栈与 Android 返回

- 只为真正的覆盖层（统一 modal / settings modal）维护前端返回栈，不为 simpleView / dashboardView 造路由——觉醒状态是业务状态，不是用户可后退的页面。
- 打开覆盖层时 `history.pushState` 写入 MicroStep 内部状态；`popstate` 关闭当前覆盖层。
- 覆盖层的 ×、取消、遮罩与提交成功关闭路径必须与 History 状态收敛，避免遗留失效历史-entry；连续打开 / 关闭多次后返回语义仍一致。
- 无覆盖层时不调用历史 API 拦截系统退出，Android 最后一次返回交还宿主 Activity。
- 设置面板内部 tab 切换不进返回栈，避免一次设置浏览产生多级后退。

### D5 · 模态框、软键盘与焦点

- 小屏下 `.modal-card` 使用 bottom sheet / 近全屏形态，宽度占满视口，顶部圆角，底部预留 safe area；桌面仍居中显示。
- 覆盖层高度以动态视口（`visualViewport.height` 或等效标准属性）为准，软键盘弹出时收缩内容区并保持可滚动。
- 聚焦输入框时将控件滚入可见区；提交 / 取消操作不得被键盘永久遮挡。
- 若目标 WebView 不支持 `visualViewport`，回退现有 CSS 最大高度与浏览器聚焦滚动，不新增插件。

### D6 · 雷达图自适应

- 以 Canvas 的 CSS 盒尺寸计算绘图坐标，继续用 `devicePixelRatio` 设置物理像素，避免模糊。
- 用 `ResizeObserver` 监听容器尺寸变化并调用现有 `renderRadar()`；尺寸未变化的事件不重复重绘。
- 小屏收缩半径与标签外边距，六维标签不得被卡片或 Canvas 边界裁剪；旋转屏幕后按新尺寸重绘。

### D7 · 工程边界与 i18n

- 新增前端辅助逻辑只使用 CSS、`history`、`visualViewport`、`ResizeObserver` 等标准 Web API。
- 所有新增用户可见文案 / aria-label 进入既有 i18n 字典，`zh-CN` / `en` key 同步。
- 不修改 `api.js`、IPC command、`Cargo.lock`、capabilities、`AndroidManifest.xml` 或 iOS `Info.plist`。

## Risks / Trade-offs

- [Android WebView 的 `popstate` / 返回语义与预期不一致] → Android 真机验收覆盖连续开关、backdrop、取消、提交后返回；若失败停止编码并修订规格，不改原生工程绕路。
- [safe-area inset 在部分 Android 机型不可用] → 所有 inset 均有 0 缺省；以真机检查手势条遮挡，不能用 iOS 模拟器结果代表 Android。
- [前端无自动化测试基建] → 静态审计 + 桌面 / 移动 viewport CDP 冒烟 + Android 真机冒烟 + `cargo test` 全量回归。
- [小屏样式可能影响桌面] → 移动规则尽量限定在 media query / coarse pointer；1280×860 与 960×640 桌面冒烟纳入验收。
- [键盘行为平台差异大] → 只验收可观察结果（聚焦输入可见、主操作可达、内容不丢失），不承诺各平台相同动画。

## Migration Plan

1. 先更新 `前端设计.md` 的移动端交互章节，再进入代码实现。
2. 以纯前端 patch 实施；每完成一个可验证阶段运行相应静态 / 冒烟检查。
3. 最终运行 `cargo test` 与 `cargo build`，再用既有 Android 安装包构建 / 安装链路真机冒烟。
4. 回滚策略：revert 前端 patch 即可；无数据迁移、无持久化 schema 变更、无原生工程变更。

## Open Questions

无。Step 1 已确认采用零构建单入口方案；iOS 真机验证受签名环境限制，规格验收以 Android 真机为强制项、iOS 响应式检查为可选补充。
