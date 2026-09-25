# tasks.md · add-settings-panel

> Step 3/4 执行版占位（TDD 红阶段：前端无测试基建，以冒烟清单 + cargo 全量回归替代；详见 design Risks）。

## 1. 设置面板骨架

- [x] 1.1 `frontend/js/settings.js`（新增）：设置按钮绑定、面板 HTML（tab 按钮组 + 三内容区）、tab 切换、宽体样式变体挂载/移除、打开时默认定位代码仓同步分类并刷新回显〔TC-SP-01~07、24〕
- [x] 1.2 `frontend/index.html`：顶栏「⚙ 设置」按钮（两视图可见）；移除 dashboardView 右栏「☁️ 远端同步」卡；脚本入口收敛为 main.js + settings.js（settings.js import sync.js）〔TC-SP-01~02、18~19、21〕
- [x] 1.3 `frontend/styles.css`：设置面板宽体模态变体 + tab 按钮组样式（沿用既有设计语言，无新字体/图标依赖）〔TC-SP-24〕

## 2. 同步表单迁移（行为等价）

- [x] 2.1 `frontend/js/sync.js` 改造：导出表单片段 HTML、事件绑定、配置回显刷新；删除 DOMContentLoaded 自绑定与仪表盘挂载点感知；语义保持：PAT 密码形态/留空保持/勾选清除、保存、立即同步前自动保存（失败中止）、最近结果展示、成功后 `loadState`〔TC-SP-08~15〕
- [x] 2.2 `settings.js` 挂载同步分类：全部内容 div 结构（非 `<form>`，规避 modalBody 统一 submit 代理误命中，design D3）；打开面板刷新一次配置（数据档案"最近同步"复用该次结果）〔TC-SP-05~06、20〕

## 3. 数据档案与关于

- [x] 3.1 数据档案分类（只读）：数据落点说明（`%APPDATA%\com.microstep.app` 口径，静态文案）+ 事件总数（已加载 state）+ 最近同步时间与结果；无任何修改入口〔TC-SP-16〕
- [x] 3.2 关于分类（静态）：应用名、产品定位、事件溯源机制一句话；零请求〔TC-SP-17〕

## 4. 文档

- [x] 4.1 `前端设计.md`：总体结构图补设置入口 + 新增「设置面板」章节（该文档要求先更新再动代码，实现首日先落）〔TC-DOC-01〕
- [x] 4.2 PROGRESS.md 进度/断点更新

## 5. 验收

- [x] 5.1 全量回归：`cargo test` 119/119 全绿、零警告（后端零改动）〔TC-REG-01〕
- [x] 5.2 `cargo build` 零警告（前端重嵌入）+ 实机重启确认面板可用〔TC-REG-02〕
- [x] 5.3 实机冒烟清单（design Risks 全四项）：两视图入口 / 三 tab 与默认定位与零请求 / 同步表单行为等价（保存回显、留空保持、勾选清除、立即同步自动保存、保存失败中止、错误 toast）/ 仪表盘旧同步卡无残留；401 分类沿用既有后端测试 TC-U21，未触碰真实远端〔TC-SP-01~24〕
