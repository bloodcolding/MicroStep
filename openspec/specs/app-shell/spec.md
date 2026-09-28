# app-shell Specification

## Purpose
TBD - created by archiving change refactor-to-tauri-v2. Update Purpose after archive.
## Requirements
### Requirement: 桌面应用打包与启动

应用 SHALL 以 Tauri v2 桌面应用形式构建与运行，目标平台为 Windows、macOS、Linux。前端 SHALL 以静态资产形式由应用内嵌托管，不依赖外部浏览器、外部 HTTP 服务或网络加载。

#### Scenario: 冷启动到可用

- **WHEN** 用户在目标平台双击启动应用
- **THEN** 主窗口加载前端单页应用且数据层就绪，全程无需手动启动任何服务或打开浏览器

#### Scenario: 无 Python 环境运行

- **WHEN** 目标机器未安装 Python 运行时
- **THEN** 应用正常启动、读写数据并完成全部业务操作（所有领域逻辑由内嵌 Rust 提供）

### Requirement: 单实例运行

应用桌面目标（Windows / macOS / Linux）SHALL 防止多实例并发运行，以避免并发写同一事件流文件造成数据损坏。移动目标 SHALL 依赖操作系统自身的应用单实例语义，SHALL NOT 注册桌面专属插件。

#### Scenario: 二次启动去重（桌面）

- **WHEN** 桌面平台上应用已在运行时用户再次启动
- **THEN** 新进程将焦点转交给既有主窗口后自行退出，不创建第二个窗口或第二个写入者

#### Scenario: 移动目标不注册桌面插件

- **WHEN** 以移动目标（`aarch64-linux-android` / `aarch64-apple-ios`）编译应用
- **THEN** single-instance 插件因编译期门控不被编译进移动产物，移动构建不受桌面专属插件影响

### Requirement: 移动端就绪约束

应用实现 SHALL NOT 使用 sidecar、子进程调用或任何桌面平台专属运行时能力（窗口配置等编译期桌面目标除外），保证后续 iOS/Android 移植无需架构级重构。桌面专属插件与代码路径 SHALL 以编译期条件（`#[cfg(desktop)]`）门控挂载，移动目标 SHALL 保持可编译。

#### Scenario: 依赖与平台调用审计

- **WHEN** 审计 Rust 依赖树与源码中的进程/平台调用
- **THEN** 不存在子进程调用、sidecar bundle 声明或运行时桌面专属 API 依赖

#### Scenario: 桌面插件编译门控

- **WHEN** 审计桌面专属能力（如 single-instance 插件）的挂载代码
- **THEN** 其注册被 `#[cfg(desktop)]` 等编译期条件包裹，桌面行为不变且移动目标编译不受阻

### Requirement: 前端零构建保留

前端 SHALL 保持原生 ES modules 无构建形态随应用分发：SHALL NOT 引入构建步骤、打包器或 npm 依赖；模块演进（新增/修改）SHALL 遵循既有前端工程约定——依赖关系无环、`api.js` 为唯一 IPC 收口、用户数据进模板前 `escapeHtml`、列表交互事件委托。迁移期"除 API 访问层外文件冻结"的等价验收条款（refactor-to-tauri-v2 一次性验收，golden replay 已归档）自此退役。

#### Scenario: 零构建审计

- **WHEN** 审计 `frontend/` 目录
- **THEN** 无 package.json / node_modules / 打包器配置 / 构建产物，JS 模块为原生 ES modules 被 index.html 直接引入

#### Scenario: 工程约定审计

- **WHEN** 前端新增或修改模块（如设置面板相关模块）
- **THEN** 模块依赖无环、IPC 调用仅出现在 api.js 收口链路、动态文案经 escapeHtml、列表类交互沿用事件委托

