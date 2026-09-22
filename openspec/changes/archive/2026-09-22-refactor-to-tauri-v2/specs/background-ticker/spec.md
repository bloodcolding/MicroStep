# background-ticker · 规格增量

## ADDED Requirements

### Requirement: 后台每日结算

应用 SHALL 在启动时检查并补注入缺失的每日 Tick 事件，并在运行期间周期性检查自然日边界、跨日时自动注入 Tick，语义与 Python 版后台 Ticker 线程等价：进入新自然日时记录前一日的最终 SAN、将 SAN 重置为 100 并继续。

#### Scenario: 启动补结算

- **WHEN** 应用在距最后一个事件跨过多个自然日后启动
- **THEN** 启动过程补齐缺失的每日 Tick 事件序列，每日一条

#### Scenario: 运行中跨零点

- **WHEN** 应用持续运行跨越自然日边界
- **THEN** 后台任务在边界后自动注入新一天的 Tick，无需用户交互

#### Scenario: 手动 Tick 兼容

- **WHEN** 用户通过 `system_tick` command 手动注入 Tick
- **THEN** 行为与 HTTP 版 `POST /api/system/tick` 一致（幂等防护与 Python 版对齐）
