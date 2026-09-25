# design.md · add-cicd-multiplatform

## Context

- 仓库托管于 GitHub（`bloodcolding/MicroStep2`，master 分支），当前无 `.github/`、无 CI/CD。
- 打包现状：本地 Windows 手工 `npx -y @tauri-apps/cli build`（README 记录）；无 Node 清单文件（无 package.json，前端零构建）。
- 架构约束：Rust 依赖白名单由 Cargo.lock 锁定（ADR-002/004）；git2 vendored-libgit2（C）+ gix/rusttls→aws-lc（C/asm）是两个 C 编译点；`src-tauri/.cargo/config.toml` 已静默 MSVC linker_messages。
- 移动端就绪基础：`lib.rs::run()` 已标 `#[cfg_attr(mobile, tauri::mobile_entry_point)]`，但 single-instance 插件**无条件注册**（桌面专属，移动构建会失败）；`data_dir` 手工 git 骨架无子进程（ADR-003 移动端就绪约束）。
- 用户实机：iOS 26.6（> TrollStore 支持上限 16.6.1/17.0，永久安装路线排除）；无 Apple 开发者账号（$99/年不做）。

## Goals / Non-Goals

**Goals**

1. push / PR 快速回归（全量 cargo test + build）。
2. 一次 tag 触发五类产物：Win NSIS+MSI、macOS universal dmg、Linux deb+rpm+AppImage、Android 签名 APK（aarch64+x86_64）、iOS 未签名 ipa。
3. 产物自动汇总 draft Release + SHA256 校验和，人工确认后 publish。
4. 移动目标保持可编译（CI 常态检查），移动构建工程入库随源码演进。

**Non-Goals**

- 前端移动端 UI 适配（viewport / 响应式 / 触控交互，另立变更）。
- Windows / macOS 桌面代码签名与公证（无证书，SmartScreen / Gatekeeper 提示照旧）。
- Apple 付费开发者账号、TestFlight、App Store / Play Store 商店分发（AAB 可日后加）。
- tauri-plugin-updater 自动更新（未装该插件，无 latest.json 需求）。
- 自建 runner / 其他 CI 平台。

## Decisions

### D1 · 裸 Tauri CLI，不用 tauri-action，不引入 package.json

workflow 直接 `npx -y @tauri-apps/cli@<钉版本> build ...`，与本地手工命令同构（排障直觉一致）；产物路径自维护 glob（`src-tauri/target/release/bundle/{nsis,msi,dmg,deb,rpm,appimage}/...`）。tauri-action 的核心增值是 updater 资产与 npm script 集成，本项目均用不上，且它默认期待 package.json——引入 node 清单与「前端零 npm」口径摩擦。CLI 版本明写于 YAML（如 `@tauri-apps/cli@2.x.y`），升级 = 改一行 + 重跑冒烟。

### D2 · 触发策略与版本守卫

- `ci.yml`：`push: branches: [master]` + `pull_request`（均触发测试作业）。
- `release.yml`：仅 `push: tags: ['v*']`。首个 job 校验 `github.ref_name == tauri.conf.json.version`（严格字符串相等，预发布号合法：tag `v0.2.1-rc1` ↔ version `0.2.1-rc1`），不一致整条流水线 fail-fast，防错发。发布动作 = 手动 bump `tauri.conf.json`+`Cargo.toml` → commit → 打同名 tag。
- concurrency：`ci.yml` 按 ref 分组 + `cancel-in-progress: true`（同分支新 push 取消旧跑）；`release.yml` 不设取消（tag 构建不应被中断）。

### D3 · 平台矩阵与 runner 选型

| 作业 | runner | 目标 / 产物 | 说明 |
| --- | --- | --- | --- |
| windows | `windows-latest` | x64：NSIS setup `.exe` + `.msi` | MSVC 预装；cmake/nasm 预装（aws-lc） |
| macos | `macos-latest` | `universal-apple-darwin` 单 `.dmg` | 一份产物双架构（Intel+AS）；编译两次但产物数最少 |
| linux | `ubuntu-22.04` | `.deb` + `.rpm` + `.AppImage` | 22.04 的 glibc 2.35 兼容面更广；apt 装 `libwebkit2gtk-4.1-dev libxdo-dev libssl-dev libayatana-appindicator3-dev librsvg2-dev cmake pkg-config` |
| android | `ubuntu-latest` | APK：aarch64 + x86_64（模拟器） | JDK 17 + 预装 Android SDK + `sdkmanager` 装 NDK；`ANDROID_HOME`/`NDK_HOME`/`JAVA_HOME` 注入 |
| ios | `macos-latest` | `--export-method debugging` 未签名 `.ipa` | 仅需 Xcode（预装），零证书零账号 |

注：若 GitHub 退役 ubuntu-22.04 镜像，回退 `ubuntu-latest`（glibc 兼容性记录于 workflow 注释）。

### D4 · Android 签名：keystore 本地生成 + GitHub Secrets 注入

- keystore 由用户本地 `keytool` 生成（README 给命令），**永不入库**；GitHub Secrets 配四项：`ANDROID_KEYSTORE_BASE64` / `ANDROID_KEYSTORE_PASSWORD` / `ANDROID_KEY_ALIAS` / `ANDROID_KEY_PASSWORD`。
- CI 优先走 Tauri/gradle 的环境变量签名注入（实现期以官方文档核对变量名）；若该路径受阻，降级为构建后用 build-tools `apksigner` 直签 unsigned release APK（等效产物）。Secrets 缺失时作业明确失败而非静默出 debug 包。

### D5 · iOS 免账号分发路径（用户 iOS 26.6）

- CI 产出 `debugging` 导出的**未签名 ipa**：构建零账号依赖；iPhone 无法直装（系统强制验签）。
- 用户侧安装：Windows 上爱思助手 / Sideloadly + 免费 Apple ID 自签 → **7 天有效期**，到期连电脑重签（约 30 秒，App 沙盒数据保留）。TrollStore（永久安装）仅支持 ≤16.6.1 / 17.0，26.6 不可用，已排除。
- README 记录完整自签操作；将来若购开发者账号，只需加证书 Secrets + 改 `--export-method`，管线结构不变。

### D6 · 两阶段交付，风险隔离

- **Phase 1（桌面）**：ci.yml + release.yml 骨架（版本守卫 + Win/mac/linux 三作业 + draft Release + 校验和）+ icns 图标补齐 + 桌面 tag 冒烟。不碰 Rust 代码。
- **Phase 2（移动）**：`lib.rs` cfg 守卫（红绿 = `cargo check --target aarch64-linux-android`）→ `tauri android init` / `ios init` 工程入库 → 移动图标集 → android/ios 作业 → 真机冒烟（Android APK 安装；iOS ipa 爱思自签装 iOS 26.6 实机）。
- Phase 2 的 C 依赖交叉编译问题不阻塞 Phase 1 已交付的桌面管线。

### D7 · 移动端 C 依赖交叉编译（最大技术风险）

- git2 `https` feature 在 unix 系拉入 openssl-sys → Android/iOS 需 vendored openssl 交叉编译（NDK/Xcode clang + perl 环境变量）；aws-lc-sys 需 cmake + NDK 工具链文件；libgit2 vendored 经 cc 交叉编译通常可行。
- 纪律：同一错误修复尝试 ≤2 次，仍失败登记 `docs/harness/errors/ERR-XXX.md` 并向用户求助；可提议降级（如该平台产物暂不含 https 同步能力的编译配置，须用户拍板），不无限期阻塞。

### D8 · 缓存与效率

- `Swatinem/rust-cache@v2`（按 Cargo.lock + 目标 triple 键控）全作业启用；Node 仅装 CLI（setup-node 钉大版本）。
- 预期：push/PR 反馈 ≤10 分钟（缓存命中后更快）；tag 全平台发布桌面阶段 ≤30 分钟、含移动端 ≤60 分钟。私有仓库 macOS 分钟 ×10 计费（可见性未确认，仅影响成本感知，不影响设计）。

### D9 · 图标与移动工程入库

- 现有图标仅 32/128 png + ico；`tauri icon` 自 128px 源放大生成全套（icns / Android mipmap / iOS Assets）——放大会有轻微模糊，属可接受取舍（不引入新设计资产）；`tauri.conf.json` icons 列表同步扩充。
- `src-tauri/gen/{android,ios}` init 后**入库**（CI 不在运行时生成移动工程，保证可复现）；`.gitignore` 现仅忽略 `src-tauri/gen/schemas/`，不影响 `gen/android`、`gen/ios`，入库前核对生成物清单。

### D10 · 文档口径切换

- AGENTS.md：「本仓库没有 lint/typecheck/formatter/CI 配置，不要自行引入」→ 修订为存在 `ci-cd` 流水线的口径（本地全量 Check 仍是 `cargo test`；CI 是补充回归）。
- README：构建/发布节双路径（本地手工 / CI tag 触发）+ iOS 自签操作 + Android 安装说明 + 产物清单。
- PROGRESS.md：移动端 UI 适配记入 Next Steps（明确非本变更范围）。

## Risks / Trade-offs

| 风险 | 应对 |
| --- | --- |
| 移动端 C 依赖交叉编译失败 | D6 两阶段隔离 + D7 ≤2 次纪律与降级路径 |
| iOS 未签名包的 7 天重签负担 | 已与用户确认接受（iOS 26.6 无 TrollStore 解） |
| 前端 UI 桌面布局装进手机 | 已知限制，README 说明 + 后续 UI 适配变更 |
| runner 镜像 / Xcode / NDK 版本漂移 | workflow 钉版本（CLI/JDK/NDK）；ubuntu-22.04 退役有 ubuntu-latest 回退 |
| gen 工程入库带来仓库体积与合并面 | gradle/xcodeproj 属生成工程低频改动，接受（换取 CI 可复现） |

## Rollout / Rollback

- **Rollout**：Phase 1 合入后以预发布版本号冒烟（bump `0.2.1-rc1` → tag `v0.2.1-rc1`，版本守卫兼容预发布号），确认三平台产物可下载安装；Phase 2 同法冒烟至真机。正式发布由用户择时 bump `0.2.1` + tag。
- **Rollback**：删除 `.github/workflows/` 与 `src-tauri/gen/{android,ios}` 即回到无 CI 状态；`lib.rs` cfg 守卫可保留（桌面行为零变化）；新增图标资产无害残留。
