# domain-rules · 规格增量

## ADDED Requirements

### Requirement: 事件重放确定性

领域核心 SHALL 保持纯函数事件重放：`applyEvent(State, Event) -> NewState` 无副作用；同一事件流在任意时刻重放 SHALL 产出完全相同的 State。既有 14 种事件 schema SHALL 字节级不变，现有真实数据 MUST 能被新实现原样重放。

#### Scenario: 幂等重放

- **WHEN** 同一份 events.jsonl 被完整重放两次
- **THEN** 两次得到的 State 序列化结果逐字节相同

#### Scenario: 真实数据兼容

- **WHEN** 以 Python 版累积的真实事件流作为输入重放
- **THEN** 全部事件被识别并正确还原 State，无未知事件类型导致的崩溃或丢弃

### Requirement: 领域规则测试覆盖移植

现有 Python 单元测试（`tests/test_domain.py` 与 `tests/test_store.py` 共 44 例）SHALL 逐条移植为 Rust 测试并保持断言语义；原 HTTP 层测试（`tests/test_server.py` 24 例）覆盖的校验语义 SHALL 由 command 层测试承接。移植后 `cargo test` 的覆盖面 SHALL NOT 低于现有用例集合。

#### Scenario: 用例对应可追溯

- **WHEN** 对照 Python 测试用例清单与 cargo test 用例
- **THEN** 每个 Python 用例在 Rust 测试中存在对应断言（以用例名或注释标注对应关系）

### Requirement: Golden Replay 等价验收

移植等价性 SHALL 以 golden replay 对照为准：迁移前由 Python 版对（a）真实事件流、（b）覆盖全部 14 种事件的构造序列分别导出 State 快照 JSON 作为基准资产提交；Rust 版重放相同输入，State JSON 归一化对比 SHALL 零差异。该基准 SHALL 保留为后续回归测试资产。

#### Scenario: 真实数据等价

- **WHEN** 用真实 events.jsonl 在 Python 版与 Rust 版分别重放并序列化 State
- **THEN** JSON 归一化（键排序、数值表示统一）后对比差异为零

#### Scenario: 全事件类型构造序列等价

- **WHEN** 用覆盖全部 14 种事件类型的构造序列在两版分别重放
- **THEN** State JSON 归一化对比零差异

#### Scenario: 基准回归常驻

- **WHEN** 后续任何领域规则修改运行测试
- **THEN** golden 基准资产可被 cargo test 消费，作为行为漂移的守门测试
