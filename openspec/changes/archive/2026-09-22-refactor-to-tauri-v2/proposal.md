# proposal.md · refactor-to-tauri-v2

## Why

当前形态（Python stdlib HTTP 服务 + 浏览器访问）要求用户手动启动服务并依赖本机 Python 环境，无法作为日常桌面应用安装、使用与分发。为支持全平台（首期 Windows/macOS/Linux，移动端进入路线图），需要迁移到 Tauri v2 桌面应用架构。

## What Changes

- **BREAKING**：Python HTTP 服务整体退役（`backend/server.py`、`run.py`、`start.bat`），替换为 Tauri v2 桌面应用，全部领域逻辑由内嵌 Rust 提供。
- **BREAKING**：14 个 HTTP API 全部退役，等价替换为 14 个 Tauri IPC command（`{ok, error, ...}` 响应信封保持兼容）。
- **BREAKING**：数据落盘从仓库内 `data/events.jsonl` 迁移到系统 AppData 目录，数据不再随 Git 仓库版本化（改由数据目录内嵌 git 仓库承接版本化路线）。
- 领域核心（domain/store，约 1,100 行）由 Python 移植为 Rust，行为等价性以 **golden replay 对照**验收（同一事件流两版重放 State 零差异）。
- 数据目录在初始化时 `git init`，为紧随其后的 Change 2（GitHub/Gitee/GitLab 远端同步）铺路。
- Python 代码库（`backend/`、`tests/`）与测试整体退役；Git 历史保留 0.0.1 时代的 Python 实现。
- 前端保持原生 ES modules 无构建形态，仅 `frontend/js/api.js` 内部实现由 fetch 改为 Tauri invoke，其余 16 个模块零改动。

## Capabilities

### New Capabilities

- `app-shell`: Tauri v2 桌面应用壳——窗口托管静态前端、单实例运行、移动端就绪约束（禁 sidecar/子进程）、前端零构建保留。
- `ipc-api`: 前后端 IPC 契约——14 个 command 与旧 HTTP API 一一映射、响应信封兼容、前端 API 层透明适配。
- `domain-rules`: 事件重放与领域规则移植——重放确定性、schema 字节级不变、测试覆盖逐条移植、golden replay 等价验收。
- `data-storage`: 数据落盘与迁移——AppData 位置、数据目录 git 化、旧数据手动迁移、损坏行容错与 mtime 缓存等价。
- `background-ticker`: 后台每日结算——启动补结算与运行期跨零点自动 Tick，语义与 Python 版 Ticker 线程等价。

### Modified Capabilities

（无——本变更是仓库首个 OpenSpec 变更，尚无既有规格。）

## Impact

- **代码**：`backend/` → 退役；新增 `src-tauri/`（domain.rs / store.rs / commands.rs / app_state.rs / ticker.rs）；`frontend/js/api.js` 内部实现替换；`tests/` 移植为 cargo test 后退役。
- **API**：HTTP 接口全部下线，前端调用面收敛到 IPC command。
- **依赖**：引入 Rust 工具链（rustup + MSVC Build Tools + WebView2）与 crates：tauri v2、serde/serde_json、tokio、tauri-plugin-single-instance。零第三方依赖原则终结（新增 ADR-002）。
- **数据**：位置迁移（新增 ADR-003）；14 种事件 schema 字节级不变，真实数据可原样重放。
- **文档**：README 运行/API 节重写；`产品方案.md`、`前端设计.md` 不受影响。
- **后续**：Change 2（Git 远端同步：远端配置 + PAT 凭据管理器 + 手动/自动 push-pull + `*.jsonl` union merge）不在本变更范围内。
