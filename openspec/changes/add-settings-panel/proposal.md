# proposal.md · add-settings-panel

## Why

同步配置目前内嵌在觉醒仪表盘右栏的「☁️ 远端同步」卡片里：① 初见期（未觉醒）用户完全无法管理同步，而启动 pull 实际对所有用户生效——入口与行为错位；② 仪表盘右栏信息密度本已偏高，同步属低频配置操作却常驻一整卡；③ 产品演进中配置类功能会逐渐增多（主题、数据、通知等），缺少统一的设置入口，每次只能往界面里再塞一张卡。

本变更引入统一的「设置」入口：顶栏专门按钮 → 模态设置面板，内部可切换查看多个类型的配置，「代码仓同步」作为首个分类从仪表盘迁入。首期共三类：代码仓同步 / 数据档案（只读）/ 关于。

## What Changes

- **新增能力 `settings-ui`**：顶栏「⚙ 设置」按钮（simpleView 与 dashboardView 均可见）打开模态设置面板（复用既有 modal 容器与关闭交互）；面板内恰好三个分类 tab 可切换（☁️ 代码仓同步 / 📦 数据档案 / ℹ️ 关于）；打开默认定位代码仓同步分类并刷新回显；tab 切换零网络请求。
- **MODIFIED `data-sync`「同步设置界面」**：同步表单从仪表盘卡片迁入设置面板「代码仓同步」分类，全部行为语义保持（URL/PAT 密码形态/分支、PAT 留空保持/勾选清除、保存、立即同步前自动保存、最近同步结果展示）；调用链仍经 `api.js` 唯一收口，IPC 契约零变化；删除迁移期「其余 17 个模块零改动」冻结条款（由 settings-ui 工程约束承接）。
- **MODIFIED `app-shell`「前端零构建保留」**：删除迁移等价验收期的文件冻结清单字面条款（"其余 16 个 JS 模块、index.html 与 styles.css 保持不变"——该清单在 add-git-remote-sync 时已有字面冲突记录），保留并强化核心不变量：原生 ES modules、无构建步骤、零 npm 依赖、前端工程约定。
- **仪表盘右栏远端同步卡整卡移除**：代码仓同步配置入口唯一化为设置面板。
- **纯前端变更**：Rust 源码、IPC command（仍 17 个）、事件 schema、`sync.json` schema 全部零改动；前端资源编译期内嵌，实现后需 `cargo build` 重启应用验证。

## Capabilities

### New Capabilities

- `settings-ui`: 统一设置入口与多类型配置查看——顶栏按钮、模态面板、三分类 tab 切换、代码仓同步表单承接、数据档案只读展示、关于静态展示、唯一配置入口、零构建工程约束。

### Modified Capabilities

- `data-sync`: 「同步设置界面」要求的挂载位置由"仪表盘卡片 + sync.js 自绑定"修订为"设置面板代码仓同步分类 + sync.js 以模块导出被 settings.js 挂载"。
- `app-shell`: 「前端零构建保留」由迁移期文件冻结清单修订为常态化的零构建/零 npm 依赖 + 工程约定约束。

## Impact

- **代码**：新增 `frontend/js/settings.js`（第 19 个 ES module）；改造 `frontend/js/sync.js`（导出表单片段与绑定，移除自绑定）；`frontend/index.html`（顶栏按钮、移除同步卡、脚本入口收敛）；`frontend/styles.css`（设置面板宽体模态 + tab 样式）。改动 ≤5 文件。
- **依赖**：零新增（前端零构建零 npm 不变；Rust 依赖树不动）。
- **数据**：零变更（`sync.json` / `events.jsonl` schema 与存储位置不动；同步启用语义维持「remote_url 非空」）。
- **文档**：`前端设计.md` 新增设置面板章节（该文档要求"先更新本文再动代码"）；PROGRESS.md 例行更新。
- **风险**：前端无自动化测试基建（且按仓库约束不引入），行为等价性靠冒烟清单验收（WebView2 实机）；`#modalBody` 的 submit 被 `modals.handleModalSubmit` 统一代理，设置面板须用 div 结构而非 form 规避误命中（design 已记）。
