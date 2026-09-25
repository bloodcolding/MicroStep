# app-shell · 规格增量

## MODIFIED Requirements

### Requirement: 前端零构建保留

前端 SHALL 保持原生 ES modules 无构建形态随应用分发：SHALL NOT 引入构建步骤、打包器或 npm 依赖；模块演进（新增/修改）SHALL 遵循既有前端工程约定——依赖关系无环、`api.js` 为唯一 IPC 收口、用户数据进模板前 `escapeHtml`、列表交互事件委托。迁移期"除 API 访问层外文件冻结"的等价验收条款（refactor-to-tauri-v2 一次性验收，golden replay 已归档）自此退役。

#### Scenario: 零构建审计

- **WHEN** 审计 `frontend/` 目录
- **THEN** 无 package.json / node_modules / 打包器配置 / 构建产物，JS 模块为原生 ES modules 被 index.html 直接引入

#### Scenario: 工程约定审计

- **WHEN** 前端新增或修改模块（如设置面板相关模块）
- **THEN** 模块依赖无环、IPC 调用仅出现在 api.js 收口链路、动态文案经 escapeHtml、列表类交互沿用事件委托
