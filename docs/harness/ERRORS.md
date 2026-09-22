# ERRORS.md · 错误索引

> **使用规则**
> - 同一错误修复尝试 ≤2 次仍失败：禁止重试，在本文件登记，然后向用户求助。
> - 本文件只放索引（一行一错）；复现步骤、日志、已尝试方案写细节文件 `docs/harness/errors/ERR-XXX.md`。
> - 会话意外中断后：读本文件 + PROGRESS.md，检查最近成功构建的 commit，向用户汇报断点。

## 错误索引

| 编号 | 日期 | 模块 | 摘要 | 状态 |
| --- | --- | --- | --- | --- |
| [ERR-001](errors/ERR-001.md) | 2026-09-22 | tests/test_domain.py | test_tick_records_history_and_resets_san 时钟敏感（仅 2026-09-19 当天可通过），KeyError '2026-09-19' | ✅ 已解决（2026-09-22 批次一选项 A：测试钉死固定日期；该 Python 测试已随 tasks 6.2 退役，Rust 侧 tests/domain.rs 承接） |
