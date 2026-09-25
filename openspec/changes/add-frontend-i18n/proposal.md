# proposal.md · add-frontend-i18n

## Why

前端全部界面文案（20 个文件、粗估 250~400 条）均为硬编码中文，无法以其他语言使用应用。作为 Tauri 桌面形态的本地优先个人工具，用户群体天然可能跨语言（自用机器语言环境、分享给非中文用户），需要界面层的多语言显示能力。同时项目硬约束明确：前端零构建、零 npm 依赖、Rust 侧不动——因此需要一个纯前端、零依赖的运行时 i18n 方案。

## What Changes

- **新增能力 `frontend-i18n`**：前端内置运行时 i18n 模块（`zh-CN` / `en` 两套字典 + 统一 `t(key, params)` 翻译函数 + 缺失 key 回退 zh-CN 并 `console.warn`）；语言选择首启跟随 `navigator.language`，手动切换后持久化于 `localStorage`（不写事件流、不写 sync.json）；顶栏新增轻量语言切换控件（两视图可见，即时切换全部可见文案，零 IPC）；静态 HTML 文案经 `data-i18n` 声明式标记批量应用（含 title / placeholder / aria-label，`<html lang>` 同步）；JS 动态文案（toast、校验提示、事件流描述、SAN 水位提示、空态与计数等）改经 `t()` 输出并支持插值；7 个白名单维度显示名由前端字典按语言提供（meta 下发名兜底）；数字 / 日期格式化 locale 随语言切换。
- **翻译边界（负面约束）**：用户数据（Task / 里程碑标题、描述、备注）与后端返回的错误消息原文 SHALL NOT 被翻译或改写，界面直接呈现原文。
- **纯前端变更**：Rust 源码、IPC command（仍 17 个）、事件 schema、`sync.json` schema、`api.js` 调用链全部零改动；语言偏好不进 AppData git 档案。前端资源编译期内嵌，实现后需 `cargo build` 重启应用验证。

## Capabilities

### New Capabilities

- `frontend-i18n`: 前端界面多语言显示——运行时字典与翻译函数、语言选择与 localStorage 持久化、顶栏切换入口、静态/动态文案本地化、维度名与数字日期格式化本地化、翻译边界与零依赖工程约束。

### Modified Capabilities

_（无。顶栏切换方案不触碰 settings-ui「恰好三类 tab」约束；零构建原生 ES module 方案吻合 app-shell「前端零构建保留」既有要求。）_

## Impact

- **代码**：新增 `frontend/js/i18n/`（index.js + locales/zh-CN.js + locales/en.js）；改造 `frontend/index.html` 与全部含用户可见文案的 JS 模块（约 19 个，机械提取，属本变更预期范围）；`frontend/styles.css` 增语言切换控件样式。
- **依赖**：零新增（前端零构建零 npm 不变；Rust 依赖树不动）。
- **数据**：零 schema 变更。唯一新增持久化为 `localStorage` 的语言偏好键；`events.jsonl` / `sync.json` 不因语言切换产生任何写入。
- **文档**：`前端设计.md` 新增 i18n 章节（该文档要求"先更新本文再动代码"）；PROGRESS.md 例行更新。
- **风险**：前端无自动化测试基建（按仓库约束不引入），完备性靠静态审计（非字典文件的中文字符残留扫描 + en 视图人工冒烟）+ WebView2 CDP 实机冒烟 + `cargo test` 全量回归兜底；切换后已打开模态的即时刷新是主要回归面（design D4 已记）。
