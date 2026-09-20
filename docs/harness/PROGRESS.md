# PROGRESS.md · 进度与待办

> **使用规则**
> - 每次会话开始（上班打卡）先读本文件 + `DECISIONS.md`。
> - 每完成一个可验证步骤立即更新本文件；下班前必须更新 Next Steps。
> - 完成的条目移入归档区并附简短总结；超过 500 行时将旧条目移入 `docs/harness/archive/`。

## 当前状态

- **日期**: 2026-09-20
- **健康度**: ✅ 68 例测试（24 HTTP + 44 单元）+ smoke 全过
- **阶段**: 重构收官后发布 tag `0.0.1`（含 Windows 一键启动 start.bat）
- **数据档案**: 全新事件流（历史数据已清空，见 ADR-001）

## Next Steps

1. （可选）路线 C：产品机制补完（疲劳惩罚/防刷递减/衰减/结项爆发，逐个走四步法，见 产品方案.md 规划蓝图）
2. （可选）方案五 dogfood：先真实使用 1-2 周，用事件流数据给路线 C 排序

## Suspended Tasks（暂存任务区）

_（无。任务切换时将未完成工作记入此处，向用户确认后切换。）_

## 归档区（结项总结）

### 2026-09-20 · start.bat 一键启动 + 发布 tag 0.0.1

- 新增 `start.bat`（纯 ASCII，规避批处理中文编码坑）：`cd /d %~dp0` 保证任意目录双击可用；python 存在性检查；后台 helper 延迟 3 秒开浏览器（等服务绑定端口）；README 运行节补充一键启动说明。
- 实测通过：隔离数据（`MICROSTEP_EVENT_PATH`）启动 → `/api/state` 200 → 杀进程清理，真实 `data/events.jsonl` 零污染（git status 验证）。
- 提交后打 annotated tag `0.0.1` 并推送 origin master。

### 2026-09-19 · 路线 B 收官：路由表驱动 + 校验收敛 + 事件类型化

- 新增 tests/test_server.py（24 例，真实 ThreadingHTTPServer + urllib），先建安全网再动刀；期间发现并修复真实 bug：非法 JSON 请求体在 try 块外未捕获导致断连，现统一 400。
- server.py 140 行 if-链 → 三张路由表（GET 精确 / POST 精确 / POST {param} 段匹配）+ 模块级 handler，统一签名 (store, body, params)。
- 校验收敛 store 层：_validate_epic_fields 共用于创建/更新；铭文 ≥2 字、Task/Epic 标题非空下沉；server 只留参数解包。
- 事件 schema TypedDict 化：14 种事件（Literal 标签），11 个工厂返回类型 + 12 个 Reducer handler 参数收紧，零运行时开销。
- 提交：9b7c24c / c5a0bf6 / 62b28c2 / 55a02b2。
- 关键难点：测试安全网先行使重构全程绿灯（期间两次编辑失误均被测试即时捕获）。

### 2026-09-19 · 重构第一批：前端模块化 + 路线 A 还债

- 前端 app.js（917 行）拆为 17 个原生 ES modules（js/ + js/render/），无构建步骤，列表交互改事件委托消除渲染层反向依赖；浏览器 QA 全过（加载/建 Task/觉醒/雷达/结算）。
- 新增 tests/test_domain.py + tests/test_store.py（42 例，stdlib unittest），期间抓到一次误删校验行的编辑事故，安全网价值实证。
- 修复前端 SAN 提示文案谎言（原描述不存在的疲劳收益惩罚机制）；产品方案.md 重构为「实现现状 + 规划蓝图」；前端设计.md 从空文件补全。
- EventStore：read_events 按 mtime 缓存（单命令从 ~6 次文件读取降到 1-2 次）；15 处 ensure 样板收敛为 _ensure_ready；MAX_EQUIPPED_TITLES 常量经 /api/meta 下发；消除 import backend.server 即创建数据文件的副作用。
- 提交：0edd3e6 / f55400c / 350d1b1 / 3dac05e / ee0ba3b。
- 关键难点：ES modules 拆分的循环依赖（actions↔controller↔render）用容器级事件委托破环；mtime 缓存需保证外部直接改文件时自动失效。

---

## 格式约定

- **当前状态**：一句话级别的事实（健康度 / 阶段 / 阻塞点），不放细节。
- **Next Steps**：可直接执行的下一步，每条带验证点。
- **Suspended Tasks**：被中断的任务，记录断点（改到哪个文件、测试状态、下一步）。
- **归档区**：每个结项一条，格式 `### YYYY-MM-DD · 任务名` + 3-6 行总结。
