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

应用 SHALL 防止多实例并发运行，以避免并发写同一事件流文件造成数据损坏。

#### Scenario: 二次启动去重

- **WHEN** 应用已在运行时用户再次启动
- **THEN** 新进程将焦点转交给既有主窗口后自行退出，不创建第二个窗口或第二个写入者

### Requirement: 移动端就绪约束

应用实现 SHALL NOT 使用 sidecar、子进程调用或任何桌面平台专属运行时能力（窗口配置等编译期桌面目标除外），保证后续 iOS/Android 移植无需架构级重构。

#### Scenario: 依赖与平台调用审计

- **WHEN** 审计 Rust 依赖树与源码中的进程/平台调用
- **THEN** 不存在子进程调用、sidecar bundle 声明或运行时桌面专属 API 依赖

### Requirement: 前端零构建保留

前端 SHALL 保持原生 ES modules 无构建形态随应用分发；除 API 访问层外，前端模块与样式 SHALL 零改动。

#### Scenario: 前端资产等价

- **WHEN** 对比迁移前后的 `frontend/` 目录内容
- **THEN** 仅 `js/api.js` 的内部实现变化，其余 16 个 JS 模块、index.html 与 styles.css 保持不变

