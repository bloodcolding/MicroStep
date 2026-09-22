# tasks.md · refactor-to-tauri-v2

> 初始占位版本（Step 2 产出）。Step 3 TDD 时将按测试方案细化为可勾选执行清单。

## 1. 环境与脚手架

- [ ] 1.1 安装并验证 Rust 工具链（rustup + MSVC Build Tools + WebView2），`cargo build` 空项目通过
- [ ] 1.2 创建 `src-tauri/` Tauri v2 脚手架：`frontendDist` 指向 `../frontend`、集成 `tauri-plugin-single-instance`、最小窗口配置
- [ ] 1.3 空壳验证：`cargo tauri dev` 启动并加载现有 `frontend/` 页面

## 2. Golden 基准（Python 侧，移植真源）

- [ ] 2.1 编写基准导出脚本：对（a）真实 `data/events.jsonl`（b）覆盖全部 14 种事件的构造序列，分别用 Python 版重放并导出 State 快照 JSON（归一化：键排序）为测试资产
- [ ] 2.2 基准资产落位 `src-tauri/tests/assets/`（或等价测试资源路径），提交入库

## 3. Rust 领域移植

- [ ] 3.1 `domain.rs`：14 种事件 enum（serde tag）、维度/称号定义、`apply_event` Reducer 与派生逻辑
- [ ] 3.2 `store.rs`：JSONL 读写、业务命令（含校验收敛）、mtime 缓存、损坏行容错、RLock→Mutex 等价
- [ ] 3.3 测试移植：`test_domain` + `test_store` 44 例逐条对应（用例名/注释可追溯），HTTP 24 例校验语义由 command 层测试承接
- [ ] 3.4 golden replay 测试：Rust 版重放两份基准输入，State JSON 归一化对比零差异

## 4. IPC 与前端接入

- [ ] 4.1 `commands.rs`：14 个 `#[tauri::command]`，参数集与响应信封（`{ok, error, ...}`）逐字段等价
- [ ] 4.2 `app_state.rs`：Store 实例托管（Mutex）+ command 层测试（信封形状、校验拒绝路径）
- [ ] 4.3 `js/api.js` 内部改 invoke：path→command 查表、body 展开、`ok:false` 抛错路径等价；其余 16 个前端模块零改动验证（diff 证明）

## 5. 数据与后台任务

- [ ] 5.1 AppData 落盘：`app_data_dir()` 定位、首次初始化（默认里程碑）、已有文件不覆盖保护
- [ ] 5.2 数据目录 `git init`（失败不阻断启动，仅告警日志）
- [ ] 5.3 `ticker.rs`：启动补结算 + tokio 周期任务跨零点注入；手动 `system_tick` 语义对齐
- [ ] 5.4 旧数据迁移文档：AppData 具体路径 + 手动拷贝步骤（README）

## 6. 退役与收尾

- [ ] 6.1 全量验收：cargo test 全绿（含 golden 回归）+ 三平台 `cargo tauri build`（至少 Windows 本机实测）+ 手工冒烟（冷启动/建 Task/结算/事件删除/迁移路径）
- [ ] 6.2 删除 `backend/`、`tests/`、`run.py`、`start.bat`（Git 历史保留）
- [ ] 6.3 README 重写（运行/构建/数据迁移）；追加 ADR-002（技术栈切换）、ADR-003（数据迁移 AppData + 同步路线）；更新 PROGRESS.md
