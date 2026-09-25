# ci-cd · 规格增量

## ADDED Requirements

### Requirement: 持续集成检查

向 master 的 push 与所有 pull request SHALL 触发持续集成工作流，在 Linux runner 上执行全量检查：`cargo test --locked` 与 `cargo build --locked`（manifest 指向 `src-tauri/Cargo.toml`）。移动端阶段落地后 SHALL 追加 `cargo check --target aarch64-linux-android --locked` 作为移动目标编译回归。同一分支新触发的运行 SHALL 取消仍在进行的旧运行。

#### Scenario: PR 全量回归

- **WHEN** 任一 pull request 更新
- **THEN** Linux runner 上运行全量 `cargo test --locked` 与 `cargo build --locked`，任一失败即该检查失败

#### Scenario: 缓存加速

- **WHEN** Cargo.lock 未变化的重复触发
- **THEN** Rust 构建缓存命中，检查耗时显著低于冷启动

#### Scenario: 旧运行取消

- **WHEN** 同一分支在旧运行未结束时再次 push
- **THEN** 旧运行被取消，以最新提交的结果为准

#### Scenario: 移动编译回归（移动端阶段后）

- **WHEN** 移动端阶段合入后的 CI 触发
- **THEN** `cargo check --target aarch64-linux-android --locked` 一并执行，移动目标编译回归纳入常态检查

### Requirement: 发布触发与版本守卫

发布工作流 SHALL 仅由 `v*` 形式的 tag push 触发（普通分支 push SHALL NOT 触发）。发布流程的首个步骤 SHALL 校验 tag 名称与 `tauri.conf.json` 的 `version` 字段严格一致（预发布号合法，如 tag `v0.2.1-rc1` 对应 version `0.2.1-rc1`）；不一致时 SHALL 立即失败且 SHALL NOT 启动任何构建作业。

#### Scenario: 版本一致放行

- **WHEN** 推送 tag `v0.2.1` 且 `tauri.conf.json` version 为 `0.2.1`
- **THEN** 版本守卫通过，五平台构建作业启动

#### Scenario: 版本不一致快速失败

- **WHEN** 推送 tag `v0.2.1` 但 `tauri.conf.json` version 为 `0.2.0`
- **THEN** 发布流程在守卫步骤失败，不消耗任何平台构建资源

#### Scenario: 分支推送不触发发布

- **WHEN** 向 master 或其他分支普通 push（无 tag）
- **THEN** 不触发发布工作流

### Requirement: 全平台产物构建

tag 发布 SHALL 并行构建五类产物，构建 SHALL 使用钉版本的 Tauri CLI（版本号明写于 workflow，SHALL NOT 使用 `latest` 或裸主版本号），cargo 命令一律带 `--locked`：

1. Windows x64：NSIS setup `.exe` 与 `.msi`；
2. macOS：`universal-apple-darwin` 目标的单个 `.dmg`（同时支持 Intel 与 Apple Silicon）；
3. Linux：`.deb`、`.rpm` 与 `.AppImage`（构建于 ubuntu-22.04 或其回退镜像，预装 webkit2gtk 等系统依赖）；
4. Android：aarch64 与 x86_64 两个 ABI 的 release 签名 APK；
5. iOS：`--export-method debugging` 导出的未签名 `.ipa`。

#### Scenario: 五类产物齐全

- **WHEN** 版本守卫通过的 tag 发布完成
- **THEN** 五类平台产物全部生成并作为 Release 资产可下载

#### Scenario: macOS 双架构单产物

- **WHEN** macOS 作业完成
- **THEN** 产出一个 universal `.dmg`，Intel 与 Apple Silicon 机器均可安装

#### Scenario: Android 双 ABI 签名

- **WHEN** Android 作业完成
- **THEN** 产出 aarch64 与 x86_64 两个已签名 release APK，均可直接安装

#### Scenario: iOS 未签名导出

- **WHEN** iOS 作业完成
- **THEN** 产出 `debugging` 导出的未签名 `.ipa`，可被爱思助手 / Sideloadly 等自签工具识别，全程不依赖任何 Apple 开发者账号或证书

### Requirement: 发布产物分发

全部构建产物 SHALL 附带 SHA256 校验和，并汇总上传到与触发 tag 关联的 **草稿** GitHub Release；Release SHALL 保持草稿状态直到人工确认后发布。发布说明 SHALL 包含产物清单与 iOS 未签名包的自签指引提示。

#### Scenario: 草稿聚合

- **WHEN** 全部构建作业成功结束
- **THEN** 所有产物与校验和文件出现在同一草稿 Release 下，等待人工检查

#### Scenario: 构建失败不发布

- **WHEN** 任一平台构建作业失败
- **THEN** 不产生残缺的草稿 Release（或草稿明确标记不完整），失败原因可在作业日志定位

### Requirement: 凭据与安全

Android 签名 keystore 及其口令 SHALL 仅经 GitHub Secrets 注入，SHALL NOT 出现在仓库任何文件中；Secrets 缺失时 Android 作业 SHALL 明确失败而非静默产出未签名包。iOS 构建路径 SHALL NOT 依赖任何 Apple 付费账号或签名证书。流水线 SHALL NOT 触碰真实用户数据（`%APPDATA%` 事件流），业务侧凭据（如同步 PAT）与流水线凭据 SHALL 严格分离。

#### Scenario: keystore 不入库

- **WHEN** 审计仓库全部文件与历史
- **THEN** 不存在 Android keystore 或其口令明文

#### Scenario: Secrets 缺失显式失败

- **WHEN** Android 签名 Secrets 未配置即触发发布
- **THEN** Android 作业在签名准备步骤明确失败并提示缺失项

#### Scenario: 无 Apple 账号依赖

- **WHEN** 审计 iOS 构建作业的全部输入
- **THEN** 不存在 Apple 开发者证书、描述文件或付费账号凭据

### Requirement: 构建可复现性

流水线 SHALL 以 Cargo.lock 锁定 Rust 依赖（全部 cargo 命令 `--locked`）、以明写版本号钉住 Tauri CLI、使用 stable Rust 工具链，并启用按 Cargo.lock 键控的 Rust 构建缓存；移动构建工程 SHALL 入库（`src-tauri/gen/{android,ios}`），CI SHALL NOT 在运行时重新生成移动工程。

#### Scenario: 依赖锁定审计

- **WHEN** 审计 workflow 中全部 cargo 调用
- **THEN** 均带 `--locked`，不存在会改写 Cargo.lock 的命令

#### Scenario: CLI 版本钉死

- **WHEN** 审计 workflow 中的 Tauri CLI 调用
- **THEN** 使用完整钉死的版本号（如 `@tauri-apps/cli@2.x.y`），不存在 `latest` 或裸主版本号

#### Scenario: 移动工程随源码演进

- **WHEN** 移动工程配置需要调整
- **THEN** 以源码提交方式修改 `src-tauri/gen/{android,ios}` 后生效，CI 运行时零生成
