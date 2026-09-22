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

## 格式约定

```markdown
## ADR-XXX · 标题
- **日期**: YYYY-MM-DD
- **状态**: Proposed | Accepted | Superseded by ADR-YYY
- **背景**: [为什么需要决策]
- **决策**: [做了什么选择]
- **影响**: [对代码 / 数据 / 使用方式的影响]
```
