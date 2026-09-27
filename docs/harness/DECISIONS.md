# DECISIONS.md · 历史架构与业务决策（ADR）

> **使用规则**
> - 会话开始必读，避免重走弯路。
> - 必须追加 ADR 的场景：引入外部依赖；修改事件 schema / 数据模型；修改公共 API 签名或删除接口；修改核心配置默认值；设计非预期 workaround。
> - 不直接修改 `Status: Accepted` 条目，如有异议先提议新 ADR。
> - 超过 500 行时将旧条目移入 `docs/harness/archive/`。

---

## ADR-001 · 清空历史事件流，重开新档案

- **日期**: 2026-09-19
- **状态**: Accepted
- **背景**: 仓库初始提交 `a9e5b48` 携带了 MicroStep 1.0 时代的真实使用数据（`data/events.jsonl` 及 3 个 `events.backup.*.jsonl` 备份）。用户决定在 MicroStep 2.0 开始全新档案。
- **决策**: 删除全部历史事件数据（含备份文件），事件流从零开始。`data/events.jsonl` 路径不变，仍纳入 Git 跟踪（新生成的数据照旧提交），不修改 `.gitignore`。
- **影响**: 所有历史属性值 / 里程碑 / Task 清零，首次启动服务将重新创建默认里程碑「无限进步」；无事件 schema 变更，Reducer 重放逻辑不受影响。

---

## ADR-002 · 技术栈切换 Tauri v2，Rust 侧统一引入依赖栈

- **日期**: 2026-09-22
- **状态**: Accepted
- **背景**: `refactor-to-tauri-v2` 变更（openspec）将应用从 Python stdlib HTTP 服务重构为 Tauri v2 桌面应用（离线优先 + 单实例 + AppData 落盘）。桌面壳、事件序列化、本地时区真实时钟与周期任务无法纯手写替代，沿用「全仓零第三方依赖」不可行。
- **决策**: 经用户统一批准引入 Rust 侧依赖（版本由 Cargo.lock 锁定）：tauri v2（2.11.6）、tauri-build、tauri-plugin-single-instance、serde / serde_json、chrono（本地时区真实时钟）、tokio（Ticker 周期任务）。工具链：rustup 1.98.1 + MSVC 14.44 + WebView2。Python 侧 `dependencies = []` 在退役（tasks 6.2）前保持不变。
- **影响**: 「零第三方依赖」原则修订为「Python 侧零依赖（至退役），Rust 侧 Cargo.lock 锁定白名单」；供应链面扩大，依赖升级须重跑全量 Check（cargo test + Python Check）；运行形态从解释器直跑变为 cargo build 产物。

---

## ADR-003 · 数据迁移 AppData + Git 同步路线

- **日期**: 2026-09-22
- **状态**: Accepted
- **背景**: `refactor-to-tauri-v2` 变更后应用为 Tauri v2 桌面形态，数据不应再落在仓库工作目录（`data/events.jsonl`）；仓库内历史真实数据已按 ADR-001 清空，存量仅为初始化/冒烟数据。spec（data-storage）要求：AppData 落盘、已有文件不覆盖保护、数据目录 git 化预留同步结构。
- **决策**: ① 事件流唯一持久化位置改为 Tauri `app_data_dir()`（identifier `com.microstep.app`：Windows `%APPDATA%\com.microstep.app`，macOS `~/Library/Application Support/com.microstep.app`，Linux `~/.local/share/com.microstep.app`）。② 旧数据迁移 = 文档指引手动拷贝（design D4：个人应用、单文件、单用户，自动导入是过度设计）；应用对已存在的事件文件不覆盖、不清空。③ 数据目录初始化时写入手工 `.git` 骨架（HEAD/config/objects/refs，无子进程调用，移动端就绪约束），不配远端、不自动提交。④ Git 远端同步（远端配置、PAT、push/pull、union merge）划入 Change 2，本变更不实现。⑤ 仓库内 `data/` 在 Python 退役（tasks 6.2）后不再被应用引用。
- **影响**: 用户数据不再随本仓库 Git 提交，数据主权转移至 AppData 下的独立 git 仓库；Change 2 前备份责任在用户（直接拷贝目录即可）；仓库瘦身为纯代码仓库；README 迁移步骤为唯一迁移入口。

---

## ADR-004 · Git 远端同步双栈：gix（fetch/对象读写）+ git2-rs（push）

- **日期**: 2026-09-24
- **状态**: Accepted
- **背景**: `add-git-remote-sync` 变更（ADR-003 ④ 既定路线）要求事件流跨设备双向同步。Step 1/2 核查确认：gix fetch 侧生产可用（Cargo / GitButler 先例）但 push 全层未实现；push 三选一（自研 send-pack / git 子进程 / git2-rs libgit2）经用户拍板选 (c) git2-rs（进程内 C 库，GitButler 生产先例组合 gix+git2）；git 子进程违反 app-shell 移动端就绪字面约束且 stderr 解析与结构化错误分类冲突。
- **决策**: ① 引入 `gix 0.87`（fetch / blob 读写 / blob/tree/commit 对象写入，纯 Rust；feature 裁剪至 `blocking-http-transport-reqwest-rust-tls` + `sha1`）与 `git2 0.20`（push，libgit2 vendored 进程内绑定；`vendored-libgit2` + `https`），版本由 Cargo.lock 锁定，构成 ADR-002 白名单扩展。② merge 在应用层（union merge 纯函数），git 仅作传输与快照历史（design D2/D3/D4）。③ 集成测试远端 = 本地 bare repo（D9）；401/403 走单测映射 + 真实远端手工冒烟。④ MSVC 链接 C 对象导致 link.exe stdout「正在创建库」被 rustc 捕获为 linker_messages 警告，以 `.cargo/config.toml` 统一 `-A linker_messages`（探针对照确认纯 Rust 项目零此警告）。
- **影响**: 依赖树 +111 crate（Cargo.lock），其中 rustls 传输后端引入 aws-lc-rs（aws-lc-sys C）与 ring——**与 data-sync 规格「除 libgit2 外 SHALL NOT 引入其他 C 依赖」字面冲突**：rustls 两大 TLS provider（aws-lc-rs / ring）均含 C/asm，纯 Rust TLS 无成熟替代；按 D1 已拍板的 GitButler 生产组合接受，待用户裁决是否修订该条款（备选：native-tls/WinSSL 仅改善 Windows）。供应链面显著扩大，依赖升级须重跑全量 Check。git2 0.20.4 的 `Remote::list` 在 0 refs 空远端有空指针 UB 检查崩溃，push 前远端复核改用 gix 握手（prepare_fetch 不 receive）绕开。

---

## ADR-005 · 移动/macOS 目标 vendored OpenSSL（依赖白名单扩展）

- **日期**: 2026-09-26
- **状态**: Accepted
- **背景**: CI android-check 在 `aws-lc-sys` 修复后推进到 `openssl-sys v0.9.117` 失败（ERR-002）：依赖链 `git2(https) → libgit2-sys → openssl-sys` 仅存在于 unix 目标（Windows 走 WinHTTP，本地开发从未暴露）；Android/iOS 无系统 OpenSSL 且 pkg-config 不支持交叉，macOS runner 的 brew openssl@3 为 keg-only 默认不可见。add-cicd-multiplatform design.md L71 已预判此风险并给出 vendored 方案；用户于 2026-09-26 批准路线 A（含 macOS）。
- **决策**: 在 Cargo.toml 增加 target-gate 直依赖 `openssl-sys = { version = "0.9", features = ["vendored"] }`（`cfg(any(android, ios, macos))`），借 Cargo feature 统一使 libgit2 拉入的同一份 openssl-sys 走源码编译；Cargo.lock 新增 `openssl-src 300.6.1+3.6.3`（仅 +1 crate，ADR-002 白名单扩展）。Windows 图中无 openssl（不受影响），Linux 桌面维持系统 libssl-dev（apt 安装，不变）。openssl-src 经 cc crate 复用 CI 已设的 `CC_/AR_<target>` NDK/Xcode 工具链，Android 目标映射 `linux-aarch64` Configure（上游刻意绕开 NDK android target 坑）；perl/make 为 runner 自带，workflow 零改动。
- **影响**: Android/iOS/macOS CI 构建时长 +2~4 分钟/目标（OpenSSL 3.x 源码编译一次并缓存）；`cargo tree` 验证目标边界——android/ios/macos 含 openssl-src、linux 仅系统链路、windows 完全无 openssl。ADR-004 的「data-sync 除 libgit2 外 SHALL NOT 引入其他 C 依赖」冲突沿用其既有豁免口径（rustls/aws-lc 同为 C，OpenSSL 为 libgit2 https 的既定代价）。

---

## ADR-006 · AppData git 骨架内置同步身份（[user] 段自给自足）

- **日期**: 2026-09-26
- **状态**: Accepted
- **背景**: CI Linux sync_engine 14 例失败（`Git("The reflog could not be created or updated")` / fetch 侧 `Network("Failed to update references...")`）。根因定位：gix ref 事务写 reflog 需 committer 签名，从 config 的 user 身份解析；**无全局 git 身份的环境 committer=None → `MissingCommitter`**，外层错误串吞掉内因。本机因 `~/.gitconfig` 有身份而全绿——已通过隔离 HOME 在本地完整复现 14 例失败（7215b60 的 logs/ 目录树修复实为无效假设：gix `should_autocreate_reflog` 本就自建父目录）。CI 全新 Linux runner 与「未安装/未配置 git 的终端用户」同属此环境，属产品级缺陷而非 CI 环境问题。
- **决策**: 骨架 config 内置 `[user] name = MicroStep / email = sync@microstep.local`（reflog committer 用）；存量骨架自愈——仅当 config 完全没有 `[user]` 段时追加，既有段落、值与 HEAD 原样保留（不覆盖）。提交对象签名不受影响（sync.rs 显式构造 "MicroStep Sync"，reflog 行身份仅为本地审计信息）。
- **影响**: AppData git 仓库不再依赖用户全局 git 身份，符合 ADR-003 移动端就绪/无外部依赖约束；tc_i26 断言新骨架含身份段 + tc_i29 钉死自愈行为（HEAD 不动、原值保留）；隔离 HOME 环境下 sync_engine 19/19 由红转绿（本地等价复现 CI 条件）。

---

## ADR-007 · 开源前隐私清理（MIT + 历史重写）

- **日期**: 2026-09-27
- **状态**: Accepted
- **背景**: 仓库准备由私有转为公开。全历史审计（工作树 + 全部 blob 扫描）确认无密钥/密码/token 泄漏，但存在两类个人信息：全部提交的作者/提交者邮箱为个人 Gmail；`data/events.jsonl` 与 golden_real 资产含真实使用数据（三个日常习惯类任务与一个创作目标类里程碑的具体标题与描述）。
- **决策**: ① 添加 MIT LICENSE（此前无许可证，默认保留所有权利，不构成开源授权）；② Git 全历史重写，作者/提交者邮箱统一改为 GitHub noreply 地址，远端 force push、tag 全部重打；③ `data/events.jsonl` 自工作树与全部历史删除；④ golden_real 事件流与状态快照中的个人信息替换为中性示例文案（保持事件类型/数量/日期/数值不动，等价性由 TC-I23 golden replay 守护，序列化对齐 Rust canonical：键排序 + 紧分隔符）；⑤ 根 `.gitignore` 新增 `keystore.properties` / `*.jks` / `*.keystore` / `*.p12` 防误提交。ADR-001 中"data/events.jsonl 路径不变、仍纳入 Git 跟踪"的约定自本 ADR 起取代。
- **影响**: 全部 commit hash 变化（本地 H:\ 主仓库与远端需重新对齐）；被清除文件的历史版本无法直接 checkout 构建（属预期）；Golden 基准的语义覆盖不受影响（仅自由文本字段被替换）。

## 格式约定

```markdown
## ADR-XXX · 标题
- **日期**: YYYY-MM-DD
- **状态**: Proposed | Accepted | Superseded by ADR-YYY
- **背景**: [为什么需要决策]
- **决策**: [做了什么选择]
- **影响**: [对代码 / 数据 / 使用方式的影响]
```
