# proposal.md · add-cicd-multiplatform

## Why

打包发布目前是纯手工动作：本地 `npx -y @tauri-apps/cli build` 一次只能出当前平台（Windows）的包，macOS/Linux 产物无处可产；移动端（Android/iOS）则完全没有打包路径——而架构自 `refactor-to-tauri-v2` 起就以「移动端就绪」为约束设计（无子进程、AppData 沙盒、手工 git 骨架），移动形态只差构建工程与流水线。用户需要一条 GitHub Actions 流水线：一次打 tag，自动产出全平台（Windows / macOS / Linux 桌面 + Android / iOS 移动）安装包并汇总为 GitHub Release。

## What Changes

- **新增能力 `ci-cd`**：两条 workflow——
  - `ci.yml`：push master / PR → Linux runner 全量 `cargo test --locked` + `cargo build --locked`（移动端阶段落地后追加 `cargo check --target aarch64-linux-android --locked` 编译回归）；
  - `release.yml`：push tag `v*` → 版本守卫（tag 与 `tauri.conf.json` `version` 强一致，不一致 fail-fast）→ 五作业并行打包 → 产物附 SHA256 校验和汇总为 **draft** GitHub Release。
- **平台矩阵**：Windows x64（NSIS `.exe` + MSI）；macOS universal（`universal-apple-darwin` 单 `.dmg` 双架构）；Linux ubuntu-22.04（deb + rpm + AppImage）；Android（aarch64 + x86_64，release 签名 APK，keystore 仅经 GitHub Secrets 注入）；iOS（`--export-method debugging` 未签名 ipa，用户本机用免费 Apple ID 经爱思助手/Sideloadly 自签侧载，无 Apple 开发者账号依赖；用户实机 iOS 26.6，TrollStore 永久安装路线不可用）。
- **MODIFIED `app-shell`「单实例运行」**：明确为桌面目标范围；移动目标依赖操作系统自身的应用单实例语义。
- **MODIFIED `app-shell`「移动端就绪约束」**：桌面专属插件（single-instance）SHALL 以 `#[cfg(desktop)]` 编译门控挂载，移动目标（`aarch64-linux-android` / `aarch64-apple-ios`）SHALL 保持可编译；无 sidecar / 子进程不变量维持。
- **前置资产补齐**：`src-tauri/icons/` 全平台图标集（icns / Android / iOS，自现有 128px 源放大生成，接受轻微模糊）；`tauri android init` / `tauri ios init` 生成 `src-tauri/gen/{android,ios}` 构建工程入库。
- **两阶段交付**：Phase 1 桌面管线（CI + 三平台打包 + Release）先行落地验收；Phase 2 移动端（构建工程 + cfg 守卫 + C 依赖交叉编译 + Android 签名 + iOS 未签名包）跟进，风险隔离互不阻塞。
- **零业务侵入**：IPC command（仍 17 个）、事件 schema、前端代码、Cargo 依赖全部零改动；Rust 侧仅 `lib.rs` 一处 `#[cfg(desktop)]` 插件守卫。

## Capabilities

### New Capabilities

- `ci-cd`: GitHub Actions 持续集成与全平台发布流水线——push/PR 测试检查、tag 版本守卫、五作业产物构建（含 Android 签名与 iOS 未签名路径）、draft Release 汇总分发、凭据安全（Secrets 注入 / 零 Apple 账号依赖）、构建可复现性（Cargo.lock + 钉版 CLI + 缓存）、移动构建工程入库。

### Modified Capabilities

- `app-shell`: 「单实例运行」明确桌面目标范围；「移动端就绪约束」新增桌面专属插件编译门控条款（`#[cfg(desktop)]`），移动目标保持可编译。

## Impact

- **代码 / 结构**：新增 `.github/workflows/{ci,release}.yml`；新增 `src-tauri/gen/{android,ios}`（init 生成的 gradle / xcodeproj 工程入库）；`src-tauri/icons/` 补全平台图标集；`src-tauri/src/lib.rs` 加一行 `#[cfg(desktop)]` 守卫；`tauri.conf.json` icons 列表扩充；AGENTS.md / README / PROGRESS.md 文档同步。
- **依赖**：Rust 依赖树与前端依赖零新增；Tauri CLI 与 GitHub Actions 仅存在于 workflow（CLI 经 `npx -y @tauri-apps/cli@<钉版本>` 使用，版本明写于 YAML），不进入 Cargo.lock、不引入 package.json。
- **数据**：零变更（事件流 / sync.json schema、存储位置、IPC 契约全不动；流水线不触碰真实用户数据）。
- **文档**：AGENTS.md「本仓库没有 CI 配置」口径修订；README 增补 CI 发布路径与 iOS 自签 / Android 安装操作；PROGRESS.md 例行更新。
- **风险**：移动端 C 依赖（vendored openssl / aws-lc / libgit2）交叉编译为最大不确定性 → 两阶段交付隔离，桌面先行；同一错误修复尝试 ≤2 次即登记 ERRORS.md 求助。前端 UI 未做移动端适配（minWidth 960 桌面布局）为**已知限制**，包可安装可运行但体验为桌面布局，UI 适配另立变更不在此夹带。私有仓库 macOS runner 分钟计费 ×10（一次发布约 30–60 分钟 mac 时间）。
