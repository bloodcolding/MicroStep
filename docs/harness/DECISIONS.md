# DECISIONS.md · 历史架构与业务决策（ADR）

> **使用规则**
> - 会话开始必读，避免重走弯路。
> - 必须追加 ADR 的场景：引入外部依赖；修改事件 schema / 数据模型；修改公共 API 签名或删除接口；修改核心配置默认值；设计非预期 workaround。
> - 不直接修改 `Status: Accepted` 条目，如有异议先提议新 ADR。
> - 超过 500 行时将旧条目移入 `docs/harness/archive/`。

---

## ADR-001 · 清空历史事件流，重开新档案

- **日期**: 2026-09-19
- **状态**: Accepted
- **背景**: 仓库初始提交 `a9e5b48` 携带了 MicroStep 1.0 时代的真实使用数据（`data/events.jsonl` 及 3 个 `events.backup.*.jsonl` 备份）。用户决定在 MicroStep 2.0 开始全新档案。
- **决策**: 删除全部历史事件数据（含备份文件），事件流从零开始。`data/events.jsonl` 路径不变，仍纳入 Git 跟踪（新生成的数据照旧提交），不修改 `.gitignore`。
- **影响**: 所有历史属性值 / 里程碑 / Task 清零，首次启动服务将重新创建默认里程碑「无限进步」；无事件 schema 变更，Reducer 重放逻辑不受影响。

---

## 格式约定

```markdown
## ADR-XXX · 标题
- **日期**: YYYY-MM-DD
- **状态**: Proposed | Accepted | Superseded by ADR-YYY
- **背景**: [为什么需要决策]
- **决策**: [做了什么选择]
- **影响**: [对代码 / 数据 / 使用方式的影响]
```
