# proposal.md · add-mobile-ui-adaptation

## Why

五平台 Release 与 Android 真机同步链路已经打通，但前端仍是 960px 起步的桌面布局：顶栏操作拥挤、触控目标偏小、模态框与软键盘交互不符合移动应用习惯，且 Android 返回手势可能绕过前端覆盖层。移动端已完成“可安装、可运行”，现在需要补齐“可用、像 Native App”的前端体验。

## What Changes

- **新增能力 `mobile-ui`**：在保持单一前端入口与零构建形态的前提下，为 iOS / Android WebView 提供小屏适配：
  - 启用 `viewport-fit=cover` 与 safe area，避开刘海屏、灵动岛和底部手势条。
  - 为小屏提供紧凑顶栏、单列卡片布局、无横向滚动的主页面，并保留 960px 以上桌面布局。
  - 提升触控可用性：可点击控件具备移动端命中区，不依赖 hover，表单输入保持可选中与可聚焦。
  - 小屏模态框改为 bottom sheet / 近全屏形态，并预留安全区。
  - 用标准 History API 接管打开中的前端覆盖层返回栈：Android 返回先关闭模态框，无覆盖层时不拦截应用退出。
  - 用 `visualViewport` 等标准 Web API 处理软键盘避让，保证聚焦输入与提交操作可见。
  - 雷达图按容器尺寸与 devicePixelRatio 自适应重绘，旋转屏幕后不裁剪。
- **明确工程边界**：不引入 UI 框架、Tailwind、npm 依赖、Tauri plugin、原生权限或双入口构建；不修改 IPC command、Rust 业务内核、事件 schema 与数据档案。

## Capabilities

### New Capabilities

- `mobile-ui`: 移动端 WebView 的响应式界面与触控体验——视口/安全区、小屏布局、触控目标、模态框返回栈、软键盘避让、Canvas 自适应，以及零依赖和平台不侵入约束。

### Modified Capabilities

_（无。`app-shell` 现有“移动端就绪约束”和“前端零构建保留”不变；本变更新增移动 UI 行为，不修改既有需求。）_

## Impact

- **代码**：预计修改 `frontend/index.html`、`frontend/styles.css`、`frontend/js/ui.js`、`frontend/js/main.js`、`frontend/js/modals.js`、`frontend/js/settings.js`、`frontend/js/render/radar.js`；可按最小实现需要新增一个零依赖前端模块（如 viewport / overlay 辅助模块）。不改 Rust 源码与 `tauri.conf.json` 桌面窗口约束。
- **依赖**：零新增。前端继续原生 ES modules；Rust / Cargo.lock / 移动原生工程依赖不变。
- **数据**：零 schema 变更；`events.jsonl`、`sync.json` 与 localStorage 既有键不新增业务数据写入。
- **文档**：实现前更新 `前端设计.md` 的移动端交互章节；实现后更新 `docs/harness/PROGRESS.md`。
- **风险**：前端无自动化测试基建且移动 WebView 行为存在平台差异；以静态审计、响应式 WebView / CDP 冒烟、Android 真机冒烟与 `cargo test` 全量回归兜底。History 返回语义若真机不符，应停止实现并修订规格，不绕路修改原生工程。
