# app-shell · 规格增量

## MODIFIED Requirements

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
