# design.md · add-frontend-i18n

## 背景与目标

前端 20 个文件硬编码中文（`index.html` + 19 个 ES module），需支持 `zh-CN` / `en` 多语言显示，且严格限前端：不动 Rust、不加 IPC、不动事件 schema、不引入任何构建步骤或 npm 依赖。

## 决策

### D1 · 方案选型：运行时字典 + `t()` + `data-i18n`（方案 A）

三方案对比：

| | A. 运行时字典 + `t()` + `data-i18n` ✅ | B. 集中重渲染（文案全部收进 render 层） | C. DOM 扫描按原文替换 |
|---|---|---|---|
| 做法 | 字典 module + 翻译函数；静态 HTML 打属性标记批量应用；JS 字符串改 `t()` | index.html 文案也改为 JS 生成，统一渲染管线 | 运行时遍历文本节点按中文原文查表 |
| 成本 | 中（机械但面广） | 高（重构渲染架构，违反外科手术原则） | 低起步 |
| 健壮性 | 高（key 缺失可检测、可回退） | 高但改动失控 | 低（插值/重渲染/属性文案必漏） |

选定 A。B 需重构整个渲染链路，远超 i18n 本职；C 对动态渲染与模板插值天然漏翻，不可验收。

### D2 · 模块结构与 key 规范

```text
frontend/js/i18n/
├── index.js          # initI18n / t / getLocale / setLocale / onLocaleChanged / applyStaticTranslations
└── locales/
    ├── zh-CN.js      # 扁平 key 字典（如 "topbar.settings": "⚙ 设置"）
    └── en.js         # 同 key 集
```

- **扁平点分层级 key**（`域.名称`，如 `topbar.awaken`、`task.empty`、`san.hint.low`），不用嵌套对象（查找与缺失检测简单直接）。
- **简单复数（i18next 式，业界标准机制）**：含计数语义的 key 提供 `.one` / `.other` 变体，`t(key, { count })` 依 `Intl.PluralRules.select(count)`（ECMA-402 标准接口）选择变体，无 count 参数时用 `key.other`。不引入完整 ICU MessageFormat（gender select 等本项目无需求）。zh-CN 仅使用 `other`（中文无复数屈折）。
- **新增语言 = 新增字典文件 + 注册**，调用方零改动。
- 模块依赖方向：`i18n/index.js` 不 import 任何业务模块（避免环）；业务模块单向依赖 i18n。

### D3 · 静态 HTML 应用机制

`index.html` 静态文案打声明式标记，由 `applyStaticTranslations()` 在启动与每次切换时批量应用：

- `data-i18n` → 元素文本内容
- `data-i18n-placeholder` → input placeholder
- `data-i18n-aria-label` → aria-label
- `<title>` 与 `<html lang>` 由 i18n 模块同步

不采用逐元素 id 手工赋值（20+ 处静态文案，声明式标记可静态审计、漏标即漏翻可被 grep 发现）。

### D4 · 语言切换刷新策略

`setLocale()` 流程：写 localStorage → `applyStaticTranslations()` → 依次通知订阅者（轻量回调列表，无框架）。订阅者：

1. `main.js` 的既有 render 管线（全量重渲染仪表盘/初见视图）；
2. `settings.js`（若设置面板打开，重渲染面板内容并保持当前 tab 与表单值语义）；
3. `sync.js` 回显区域随 settings 重渲染刷新。

瞬时文案（toast）不回翻——生命周期秒级，属可接受边界，design 风险区已记。**已打开模态的即时刷新是主要回归面**，冒烟清单必须覆盖。

### D5 · 维度名与数字/日期格式化

- `dimensionName(key)` 改为：i18n 字典（7 个白名单 key）→ `getMeta()?.dimensions?.[key]?.name` 兜底 → key 原文。后端 meta 仍下发中文名，但前端字典优先覆盖。
- `toLocaleString("zh-CN")` 等硬编码 locale 收敛为 `currentNumberLocale()`（zh-CN → `zh-CN`，en → `en-US`）；日期时间格式化（`formatDateTime`）同样跟随。

### D6 · 语言选择与持久化

- localStorage 键：`microstep.locale`，值 `zh-CN` / `en`。
- 解析优先级：localStorage 合法值 → `navigator.language`（`zh*` 前缀 → `zh-CN`，否则 `en`）→ 最终兜底 `zh-CN`。
- 语言偏好不写入事件流、不写入 sync.json、不进 AppData git 档案（WebView2 localStorage 随用户数据目录走，属壳层状态）。

### D7 · 翻译边界

**不翻译**：用户数据（Task / 里程碑标题、描述、铭文、备注——用户怎么写就怎么显示）；后端返回的错误消息原文（如同步 401 文案，逐条映射脆弱且无尽头，保持原样直显）。前端自身生成的校验、提示、空态文案全部翻译。

### D8 · 与业界 i18n 方案的对齐口径

核心机制对齐主流库（i18next / FormatJS）的通行形态：key 字典 + 插值 + 回退链 + localStorage/navigator 检测顺序 + `data-i18n` 声明式绑定 + `Intl` 数字日期格式化 + `Intl.PluralRules` 简单复数。刻意差异两条：① 零依赖手写（ADR-002 前端零 npm 约束）；② 不做完整 ICU MessageFormat（无 gender/select 需求，简单复数已覆盖英文计数文案）。两套字典以 ES module 内嵌不懒加载（桌面内嵌 + 仅 2 语言，合理）。

## 风险与验证策略

- **前端无测试基建**（按仓库约束不引入）：以 ① 静态审计（非字典文件的用户可见中文字符残留扫描；en 字典 key 与 zh-CN 字典 key 集合一致性）② WebView2 CDP 实机冒烟清单 ③ `cargo test` 全量回归（后端零改动应保持 119/119）三层验收。
- **编译期内嵌**：每次前端改动需 `cargo build` 后重启应用验证，冒烟在最终态一次执行。
- **遗漏 key**：运行时回退 zh-CN + `console.warn("AGENT-I18N:", key)` 便于冒烟当场发现（遵循前端调试前缀约定）。
- **回滚**：纯前端 revert 即可；localStorage 残留键无副作用。
