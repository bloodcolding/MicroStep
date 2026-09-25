# design.md · add-settings-panel

## Context

现状：顶栏（`index.html`）仅「立即觉醒 / 刷新状态」两按钮，无设置入口；同步表单内嵌 dashboardView 右栏卡片（remote URL / PAT password / 分支 / 清除 PAT / 保存 / 立即同步 / 最近结果），由 `frontend/js/sync.js` 以独立 module script 自绑定 DOMContentLoaded 实现；初见期用户看不到同步配置。既有 modal 体系：`ui.js#openModal(title, bodyHtml)` 写入 `#modalBody`，`main.js` 对 `#modalBody` 统一代理 submit 到 `modals.handleModalSubmit`、对 `#modal` 代理 `[data-close-modal]` 关闭。Step 1 Brainstorming 四问已全部落定（2026-09-25 用户采纳全部推荐）：Q1(a) tab 切换语义、不加 enabled 字段；Q2 首期三分类；Q3 仪表盘同步卡整卡移除；Q4 顶栏按钮。

## Goals / Non-Goals

**Goals**

- 统一设置入口：顶栏按钮 + 模态面板，觉醒前后均可用（与"启动 pull 对所有用户生效"对齐）
- 多类型配置可切换：首期三类（代码仓同步 / 数据档案 / 关于），结构可渐进扩展
- 同步配置迁移零语义漂移：表单行为、PAT 处理、调用链、IPC 契约全部不变
- 纯前端、零依赖、改动收敛（≤5 文件）

**Non-Goals**

- 不加「同步启用/停用」开关（维持 remote_url 非空 = 启用的隐式语义；显式 enabled 字段属 `sync.json` schema 变更，需另行 ADR）
- 不做多 provider 配置档（GitHub/Gitee 多套配置一键切换，另立变更）
- 不新增主题/通知等配置类型（首期仅数据档案/关于两个轻量分类占位，不预制空壳框架）
- 不动 Rust / IPC / 事件与配置 schema；不做设置持久化（面板状态不落盘，每次打开即取现值）

## Decisions

### D1. 复用既有 modal 容器 + 宽体样式变体（排除独立设置视图/自绘浮层）

设置面板用 `openModal("设置", ...)` 挂入既有 `#modal`，关闭交互（× / 遮罩 / 取消）免费复用；新增 `.modal-card` 宽体变体 class（settings.js 打开时挂上、关闭时移除），tab 用模态内按钮组实现。独立设置视图（第三 view）对低频配置操作过重，且要动视图路由；自绘浮层违背"与现有模态交互一致"。

### D2. 模块分工：settings.js（面板骨架与 tab）+ sync.js（同步表单内聚）

新增 `settings.js`：负责设置按钮绑定、面板 HTML（tab 按钮 + 三内容区）、tab 切换、数据档案/关于渲染、打开时刷新同步回显。`sync.js` 改造为**导出**表单片段 HTML、绑定函数与刷新函数，由 settings.js 挂载到「代码仓同步」内容区；`sync.js` 不再自绑定 DOMContentLoaded、不再感知挂载位置。后续新增配置类型只动 settings.js，sync 职责内聚不动。脚本入口收敛为 `main.js` + `settings.js`（settings.js import sync.js），index.html 删除原 sync.js script 标签。

### D3. 同步 tab 用 div 结构而非 `<form>`（规避 modalBody 统一 submit 代理）

`main.js` 将 `#modalBody` 的 submit 统一代理到 `modals.handleModalSubmit`：未命中 epic/task/complete-epic 分支的 form 提交仍会走到末尾 `closeModal() + loadState()`，造成设置面板误关闭。设置面板全部内容（含同步表单）用 div + button 结构（现状同步卡本就是 div + button，无行为变化）。

### D4. 数据档案/关于 tab 只读且零新增 IPC

数据档案：数据落点说明文案（`%APPDATA%\com.microstep.app`，与 ADR-003/README 口径一致，静态写死不新增 command）、事件总数（取已加载 `state.events` 长度，打开面板时读取）、最近同步时间与结果（复用 `fetchSyncConfig` 信封）。关于：应用名、产品定位、事件溯源一句话（纯静态）。两个 tab 均 SHALL NOT 提供修改入口。

### D5. tab 切换零网络请求

打开面板时刷新一次同步配置回显（含数据档案"最近同步"字段一并取自该次结果）；此后 tab 来回切换纯本地显隐，不发任何请求。同步操作（保存/立即同步）成功后按既有逻辑刷新回显与 `loadState`。

### D6. app-shell 文件冻结条款正式退役

「其余 16 个 JS 模块、index.html 与 styles.css 保持不变」是 refactor-to-tauri-v2 的迁移等价验收条款（golden replay 已归档、等价性已验收），add-git-remote-sync 时已出现字面冲突（api.js +3 路由、index.html 挂载点、新增 sync.js）并在 tasks 6.2 记录待裁决。本变更将其修订为常态化约束：零构建、零 npm 依赖、原生 ES modules、前端工程约定（依赖无环 / api.js 唯一 IPC 收口 / escapeHtml / 事件委托）。存量 data-sync「其余 17 个模块零改动」场景同步删除，避免双份冻结条款。

## Risks / Migration

- **无自动化测试**：前端无测试基建且仓库约束禁止引入构建/CI。验收 = `cargo test` 全量回归（后端零改动应 119/119 不变）+ `cargo build` 零警告（前端重嵌入）+ 实机冒烟清单：①两视图设置入口可见可开；②三 tab 齐全、默认同步 tab、来回切换无请求（DevTools/CDP 观测）；③同步表单行为等价（保存回显脱敏、留空保持、勾选清除、立即同步自动保存、错误 toast、成功后状态刷新）；④未觉醒视图无同步卡残留、觉醒后右栏同步卡不存在。
- **回归面极小**：Rust 零改动，事件流与 sync.json 不触碰；同步失败路径文案与错误分类沿用 data-sync 既有信封。
- **样式风险**：宽体模态在窄窗口下的表现，冒烟时确认最小可用宽度（桌面目标平台）。

## Open Questions

无（Step 1 四问已全部落定）。
