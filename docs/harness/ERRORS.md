# ERRORS.md · 错误索引

> **使用规则**
> - 同一错误修复尝试 ≤2 次仍失败：禁止重试，在本文件登记，然后向用户求助。
> - 本文件只放索引（一行一错）；复现步骤、日志、已尝试方案写细节文件 `docs/harness/errors/ERR-XXX.md`。
> - 会话意外中断后：读本文件 + PROGRESS.md，检查最近成功构建的 commit，向用户汇报断点。

## 错误索引

| 编号 | 日期 | 模块 | 摘要 | 状态 |
| --- | --- | --- | --- | --- |
| [ERR-001](errors/ERR-001.md) | 2026-09-22 | tests/test_domain.py | test_tick_records_history_and_resets_san 时钟敏感（仅 2026-09-19 当天可通过），KeyError '2026-09-19' | ✅ 已解决（2026-09-22 批次一选项 A：测试钉死固定日期；该 Python 测试已随 tasks 6.2 退役，Rust 侧 tests/domain.rs 承接） |
| [ERR-002](errors/ERR-002.md) | 2026-09-26 | ci.yml android-check | openssl-sys 在 aarch64-linux-android 下无系统 OpenSSL（前置 aws-lc-sys 错误已由尝试 2 修复，此为新错误类） | ✅ 已解决（路线 A 经用户批准：vendored openssl + openssl-src 入锁，ADR-005） |
| [ERR-003](errors/ERR-003.md) | 2026-09-26 | ci.yml test（Linux） | sync_engine 14 例失败：无全局 git 身份环境 gix reflog MissingCommitter（错误串 "The reflog could not be created or updated" / "Failed to update references..."） | ✅ 已解决（骨架 [user] 身份自给自足 + 存量自愈，本地隔离 HOME 复现红→绿，ADR-006） |
