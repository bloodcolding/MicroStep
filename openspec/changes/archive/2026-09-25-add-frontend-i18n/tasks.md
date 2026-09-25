# tasks.md · add-frontend-i18n

> Step 3/4 执行版占位（TDD 红阶段：前端无测试基建，以静态审计 + 冒烟清单 + cargo 全量回归替代；详见 design 风险区）。

## 1. i18n 核心模块

- [x] 1.1 `frontend/js/i18n/index.js`（新增）：`initI18n`（语言解析：localStorage → navigator.language → zh-CN 兜底）、`t(key, params)` 插值、`Intl.PluralRules` 简单复数（`.one` / `.other`，无 count 用 `.other`）、缺失 key 回退 zh-CN + `console.warn("AGENT-I18N:", key)`、`getLocale` / `setLocale` / `onLocaleChanged` 订阅、`applyStaticTranslations`（data-i18n 系列 + html.lang + title）
- [x] 1.2 `frontend/js/i18n/locales/zh-CN.js` + `en.js`：扁平 key 字典骨架（首期覆盖全部提取文案；两文件 key 集合一致）

## 2. 切换入口与静态改造

- [x] 2.1 `frontend/index.html` 顶栏语言切换控件（两视图可见）+ 全部静态文案打 `data-i18n` / `data-i18n-placeholder` / `data-i18n-aria-label` 标记
- [x] 2.2 `frontend/styles.css`：切换控件样式（沿用顶栏 ghost-button 设计语言，无新依赖）
- [x] 2.3 `main.js` 启动时 `initI18n` + 绑定切换控件 + 订阅语言变化触发全量重渲染

## 3. 动态文案提取（机械改造）

- [x] 3.1 render/ 子目录（tasks / epics / events / cards / titles / radar / dashboard / simple）
- [x] 3.2 `modals.js` / `actions.js`（表单、校验提示、确认弹窗、toast）
- [x] 3.3 `settings.js` / `sync.js`（面板三分类、同步表单、回显文案）
- [x] 3.4 `dimensions.js` / `state.js` / `controller.js` / `ui.js` / `utils.js` 残余用户可见文案

## 4. 维度名与格式化

- [x] 4.1 `dimensionName`：i18n 字典（7 白名单 key）→ meta 名兜底 → key 原文
- [x] 4.2 数字 / 预览 / 日期格式化 locale 跟随（`zh-CN` ↔ `en-US`），收敛硬编码 `toLocaleString("zh-CN")`

## 5. 文档

- [x] 5.1 `前端设计.md`：新增 i18n 章节（先更新本文再动代码，实现首日先落）
- [x] 5.2 PROGRESS.md 进度 / 断点更新

## 6. 验收

- [x] 6.1 静态审计：非字典文件无残留用户可见硬编码中文；zh-CN / en 字典 key 集合一致；`api.js` 与 IPC 调用链零改动（17 command 不变）
- [x] 6.2 全量回归：`cargo test` 119/119 全绿、零警告（后端零改动）
- [x] 6.3 `cargo build` 零警告（前端重嵌入）+ 实机重启
- [x] 6.4 实机冒烟清单（CDP）：首启跟随系统 / 手动切换持久化 / 非法 localStorage 回退 / 两视图入口可达 / 切换即时刷新（含已打开设置面板）/ 缺失 key 回退与 console.warn / 维度名与数字日期 en 形态 / 用户数据与后端错误原文直显 / 事件流零写入
