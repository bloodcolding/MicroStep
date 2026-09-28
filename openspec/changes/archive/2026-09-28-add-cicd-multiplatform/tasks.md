# tasks.md · add-cicd-multiplatform

> Step 3/4 执行版占位（TDD 红绿口径：workflow 层以语法校验 + tag 冒烟清单验收；Rust 守卫以移动目标编译检查为红绿；详见 design D6/D7）。

## 1. Phase 1 · CI 与桌面发布

- [x] 1.1 `.github/workflows/ci.yml`（新增）：push master / PR → ubuntu 全量 `cargo test --locked` + `cargo build --locked`（`--manifest-path src-tauri/Cargo.toml`）；`Swatinem/rust-cache`；concurrency 按 ref 取消旧跑
- [x] 1.2 `.github/workflows/release.yml`（新增）：tag `v*` 触发；首个作业做版本守卫（`github.ref_name` 严格等于 `tauri.conf.json.version`，不一致 fail-fast，下游构建不启动）
- [x] 1.3 release 桌面三作业：`windows-latest`（NSIS `.exe` + MSI）/ `macos-latest`（`--target universal-apple-darwin` 单 dmg）/ `ubuntu-22.04`（apt 系统依赖 + deb/rpm/AppImage）；Tauri CLI 版本钉写于 YAML（`TAURI_CLI_VERSION: 2.11.4` 单源钉版）
- [x] 1.4 draft Release 聚合：各作业产物附 SHA256 校验和，统一上传 tag 关联的草稿 Release（不自动 publish）
- [ ] 1.5 图标补齐：`tauri icon` 自现有 128px 源生成全套 → `src-tauri/icons/`（含 icon.icns）+ `tauri.conf.json` icons 列表扩充；实测 macOS dmg 打包是否强制 icns 并记录
- [ ] 1.6 桌面冒烟：bump 预发布版本（如 `0.2.1-rc1`）→ 打 tag → 验证三平台产物可下载、Windows 本机可安装运行、dmg/deb/AppImage 完整性（下载校验 SHA256）

## 2. Phase 2 · 移动端

- [x] 2.1 `src-tauri/src/lib.rs`：single-instance 插件挂载加 `#[cfg(desktop)]` 守卫（红：TC-S01 失败 + 本机 check 因 NDK 缺失先败；绿：守卫后 cargo test 121/121 通过，桌面行为零变化；权威移动编译绿在 CI）
- [x] 2.2 `tauri android init` + `tauri ios init`；`src-tauri/gen/{android,apple}` 入库（本机无 NDK/Xcode → 经 `mobile-gen.yml` 手动触发生成回传），核对 `.gitignore` 与生成物清单（design D9）
- [ ] 2.3 移动图标集：`tauri icon` 产物接入 Android mipmap / iOS Assets
- [x] 2.4 release.yml 增 Android 作业：JDK 17 + Android SDK/NDK + `ANDROID_HOME`/`NDK_HOME`/`JAVA_HOME`；`--apk` 出 aarch64 + x86_64；keystore 四项 Secrets 注入签名（官方 keystore.properties 路径 + Secrets 缺失显式失败 + apksigner 验签，design D4）
- [x] 2.5 release.yml 增 iOS 作业：`macos-latest` + `tauri ios build --export-method debugging` 出未签名 ipa（零证书依赖）
- [x] 2.6 ci.yml 增 `cargo check --target aarch64-linux-android --locked` 常态移动编译回归（钉版 NDK 27.1.12297006）
- [ ] 2.7 README 落 keystore 本地生成命令（keytool）与 Secrets 配置指引（文档已完成；用户本地生成并配好四项 Secrets 待办）
- [ ] 2.8 移动冒烟：tag 重打 → Android APK 真机安装启动读写数据；iOS ipa 经爱思助手免费 Apple ID 自签安装 iOS 26.6 实机（7 天时效口径确认）

## 3. 文档

- [x] 3.1 AGENTS.md：修订「没有 CI 配置不要自行引入」口径（本地全量 Check 仍 `cargo test`；CI 为补充回归），速查表补 CI 触发说明
- [x] 3.2 README：构建/发布节双路径（本地手工 / CI tag）、产物清单、Android 安装、iOS 自签操作（design D5）
- [ ] 3.3 PROGRESS.md 更新；「移动端 UI 适配」记入 Next Steps（非本变更范围）（随收尾持续更新，结项时勾选）

## 4. 验收

- [x] 4.1 全量回归：`cargo test`（src-tauri）全绿 121/121 + `cargo build` 零新增警告
- [x] 4.2 `openspec validate add-cicd-multiplatform --strict` 通过
- [ ] 4.3 全平台 tag 冒烟：五类产物齐全（Win exe/msi、mac dmg、Linux deb/rpm/AppImage、Android 签名 APK ×2 ABI、iOS 未签名 ipa）+ SHA256 校验和 + draft Release
- [ ] 4.4 `graphify update .` 同步知识图谱
