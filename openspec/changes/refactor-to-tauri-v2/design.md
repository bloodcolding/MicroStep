# design.md · refactor-to-tauri-v2

## Context

MicroStep 2.0 当前为 Python 3.12+ 纯标准库实现：`backend/domain.py`（647 行纯函数 Reducer）、`backend/store.py`（465 行 JSONL 事件流 + 业务命令）、`backend/server.py`（283 行路由表 HTTP 服务）+ 原生 JS 无构建前端（17 个 ES modules）。事件流 `data/events.jsonl` 位于仓库内并纳入 Git。

用户决策（Step 1 已批准）：首期桌面三端（Win/macOS/Linux）、架构移动就绪；Python 彻底退役；数据迁 AppData 并走 Git 同步路线（同步为 Change 2）；两阶段交付。本变更 = Change 1（重构 + Rust 移植）。

关键有利条件：事件溯源 + 纯函数 Reducer 使「golden replay 对照」成为免费的完美等价性测试；`frontend/js/api.js` 是唯一 fetch 收口点（23 行），保持导出签名即可让前端其余部分零改动。

## Goals / Non-Goals

**Goals:**

- Tauri v2 桌面应用，Win/macOS/Linux 可构建运行，无 Python 依赖
- 领域逻辑 Python → Rust 行为等价（golden replay 零差异验收）
- 14 个 HTTP API → 14 个 IPC command，响应信封兼容，前端渲染层零改动
- 数据落 AppData + 数据目录 git init（为 Change 2 铺路）
- Python 代码与测试退役，仓库转纯 Tauri 形态

**Non-Goals:**

- Git 远端同步（GitHub/Gitee/GitLab 配置、PAT、push/pull、union merge）——Change 2
- 移动端（iOS/Android）构建与 UI 适配——路线图后续
- 自动更新机制、代码签名、商店分发
- 领域规则变更或新功能（纯行为等价移植）
- 前端 UI/UX 改版

## Decisions

### D1. 后端 Rust 全量移植（排除 Python sidecar）

移动端在路线图上，而 Tauri 移动端不支持子进程/sidecar → sidecar 方案自断后路。备选：sidecar（PyInstaller 打包，体积 +50~100MB，双工具链维护，移动不可达）——否决。

### D2. 金样重放对照移植（排除按文档重写 / 批量转译）

行为漂移是移植最大风险（clamp 边界、称号加成结算时机、SAN 日结与补结算时刻、tombstone 语义、损坏行容错等细节极易走样）。Python 版先对（a）真实事件流（b）覆盖 14 种事件的构造序列导出 State 快照 JSON 作为基准资产；Rust 版重放相同输入，归一化对比零差异为验收门禁。备选：按 README/产品方案重写（漂移无护栏）、LLM 批量转译（核验成本≈手工且隐性偏差更难发现）——均否决。

### D3. IPC 信封兼容 + api.js 透明适配

Rust command 返回与 HTTP 版逐字段等价的 `{ok, error, ...}` 信封；`api.js` 保持 `api(path, options)` 导出签名，内部查表把 `/api/...` 路径映射为 invoke 调用、把 body 展开为参数、把 `ok:false` 抛为异常。收益：17 个前端模块零改动，前端回归成本≈0。备选：前端全面改写为显式 invoke 调用——改动面大且无行为收益，否决。

### D4. 数据落 AppData + 初始化时 git init

标准桌面惯例；`git init` 不配远端不自动提交，仅让数据目录成为仓库，Change 2 直接在其上加同步。旧数据迁移 = 文档指引手动拷贝（个人应用、单文件、单用户，自动导入是过度设计）。

### D5. Ticker 用 tokio 后台任务

Python 版为后台线程周期检查。Tauri 生态标准异步运行时为 tokio，启动补结算放在 setup 钩子，运行期周期检查用 tokio::spawn 定时任务。备选：std::thread——与 Tauri 异步命令生态割裂，否决。

### D6. 事件模型用 serde 强类型

14 种事件以 `#[serde(tag = "type")]` 的 enum 建模（对应 Python TypedDict + Literal 工厂），未知类型反序列化失败的容错语义与 Python 版「损坏行跳过」对齐（实现时以 store.py 现行为准并记入测试）。schema 字节级兼容由 golden replay 资产守卫。

### D7. 单实例插件

`tauri-plugin-single-instance` 防多实例并发写。多窗口/多进程并发 append 是 JSONL 损坏的现实来源，Python 版未防护，本次顺带补上（成本一个插件声明）。

### D8. 前端零构建接入

`tauri.conf.json` 的 `frontendDist` 直接指向 `../frontend`，无 devUrl/build 配置——保持「前端改动刷新即生效」的开发体验（`cargo tauri dev` 下）。

## Risks / Trade-offs

- [Rust 工具链未安装，Windows 需 MSVC Build Tools + WebView2] → 首个任务即安装验证 `cargo build`，环境问题前置暴露
- [移植行为漂移] → D2 golden replay 全程兜底；44 例测试逐条移植提供细粒度定位
- [SAN 日结时区/补结算细节口径复杂] → 不靠推断，以 Python 版现行为唯一真源，golden 快照固化
- [git init 在受限环境失败的边界情况] → git init 失败不阻断应用启动（仅记录告警，Change 2 再补用户可见提示）
- [数据迁移靠手动拷贝存在用户失误风险] → 文档写明路径 + 「不覆盖已有文件」保护（specs 已约束）
- [仓库形态大改的回滚成本] → 迁移完成前 Python 版与 Rust 版短期共存于仓库，golden 全绿 + 三平台构建通过后再删除 Python（任务序列保证）

## Migration Plan

1. 环境：安装 Rust 工具链并验证构建
2. Python 侧产出 golden 基准资产（真实流 + 构造序列快照）
3. Rust 移植：domain.rs → store.rs → 测试移植 → golden 全绿（此期间 Python 版仍是可运行真源）
4. Tauri 壳：脚手架、commands、ticker、single-instance、api.js 适配
5. 数据：AppData 落盘 + git init + 迁移文档
6. 退役：删除 `backend/` `tests/` `run.py` `start.bat`，重写 README，追加 ADR-002/003
7. 验收：cargo test 全量 + golden 回归 + 手工冒烟（冷启动/建 Task/结算/迁移路径）

回滚策略：步骤 3-5 期间任何不可解问题可直接丢弃 `src-tauri/`，Python 版完好；步骤 6 删除 Python 前要求 golden + 构建 + 冒烟全绿。

## Open Questions

- SAN 日结的时区与补结算精确口径：实现 D2 基准时从 `domain.py`/`store.py` 现行为提取固化，不另行设计（如发现 Python 版自身存在未定义行为，停下来向用户报告再定）
- `tauri.conf.json` 各平台窗口细节（最小尺寸、标题等）：实现时对齐现有前端设计文档，不影响规格
