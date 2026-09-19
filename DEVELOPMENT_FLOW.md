## 四步法：设计 → 规格 → 测试 → 实现（带门禁 + 会话隔离）

### 总则：强制遵循的三条纪律

1. **会话隔离**：Step 1+2 一个会话，Step 3+4 新会话（仅加载规格）。
2. **门禁审批**：每一步产出后必须获得用户明确批准（`APPROVED` / `TEST_PLAN_APPROVED`）。
3. **全量回归**：实现完成后必须运行全量测试，不允许只测新增部分。

---

## Step 1：Superpowers Brainstorming → 结构化设计摘要

### 目标
将用户模糊需求转化为**多方案对比 + 推荐方案 + 可被 OpenSpec 直接消费的设计摘要**，避免与后续 Step 2 职责重叠。

### 触发方式
- 自然语言触发：“头脑风暴 / 做新功能 / 方案对比”
- 斜杠命令：`/brainstorm`

### AI 行为
1. 通过苏格拉底式提问澄清需求（技术栈、约束、非功能性要求）。
2. 提出 **2-3 种方案**，每个方案包含：实现成本、安全性、用户体验、推荐指数。
3. 推荐最优方案。
4. **输出固定格式的设计摘要**（作为 Step 2 的输入）：

```markdown
## DESIGN_SUMMARY_FOR_OPENSPEC
- **选定方案**: [方案名称]
- **核心实体**: [新增/修改的实体]
- **API 端点**: [关键接口路径]
- **技术依赖**: [库/框架]
- **关键约束**: [超时、限流、第三方限制等]
- **非功能需求**: [性能、并发、安全]
```

### 产出物
- 设计方案对比文档（可临时）
- `DESIGN_SUMMARY_FOR_OPENSPEC` 区块（必需）

### 门禁 🚪
用户必须**明确回复 `APPROVED`** 才能进入 Step 2。若要求修改，AI 调整后重新提交审批。

---

## Step 2：OpenSpec Propose → 正式规格

> ⚠️ **会话隔离**：建议在 **新的对话窗口** 中执行 Step 2，只粘贴 Step 1 产出的 `DESIGN_SUMMARY_FOR_OPENSPEC`，不保留原始对话历史。

### 目标
将设计摘要转化为 OpenSpec 的规范文档（`proposal.md`、`spec.md`、`design.md`、`tasks.md`）。

### 触发方式
- 斜杠命令：`/opsx:new <change-name>` + `/opsx:continue`（或 `/opsx:ff` 全量生成）
- 或在对话中要求：“请根据以下设计摘要，使用 OpenSpec 创建变更提案”

### AI 行为
1. 读取用户提供的 `DESIGN_SUMMARY_FOR_OPENSPEC`。
2. 执行 `openspec propose` 流程：
    - 在 `openspec/changes/<change-name>/` 下生成 `proposal.md`（动机、影响、风险）。
    - 在 `specs/<capability>/spec.md` 中生成规格增量（使用 `## ADDED|MODIFIED|REMOVED Requirements`）。
    - 可选生成 `design.md`（架构、数据模型、API 详细设计）。
    - 生成初始 `tasks.md`（仅占位，后续由 Superpowers 填充）。
3. 输出摘要，请求用户评审。

### 产出物
- OpenSpec 变更目录下的所有正式文档。

### 门禁 🚪
用户执行 `openspec validate <change-name>` 验证格式，评审内容无误后**回复 `APPROVED`** 才能进入 Step 3。

---

## Step 3：Superpowers TDD → 测试方案 + 测试代码（分离门禁）

> ⚠️ **会话隔离**：再次**新建对话**，只 `@` 引用 `openspec/changes/<change-name>/` 下的规格文件，不携带历史。

### 目标
根据规格产出**先测试方案 → 评审通过 → 再写测试代码**，确保测试覆盖完整且与规格一致。

### 子步骤 3.1：输出测试方案（仅方案，不写代码）

**AI 行为**：
- 读取 `spec.md` 和 `design.md`。
- 输出结构化测试方案，包含：
    - 单元测试覆盖点（业务逻辑）
    - 集成测试覆盖点（API 层）
    - 边界条件 / 异常路径 / 并发场景（如适用）
- 使用固定格式：

```markdown
## TEST_PLAN
### 单元测试
- TC-U01: [场景描述] → 预期结果
- TC-U02: ...

### 集成测试
- TC-I01: ...

### 边界/异常
- TC-E01: null 输入 → 400
- TC-E02: 超时 → 降级

### 评审清单（供用户勾选）
- [ ] 覆盖了正常路径
- [ ] 覆盖了异常路径
- [ ] 覆盖了边界条件
- [ ] 考虑了并发（如需要）
- [ ] 断言与规格一致
```

**门禁 🚪**  
用户使用评审清单检查后，回复 **`TEST_PLAN_APPROVED`** 才能进入子步骤 3.2。

### 子步骤 3.2：编写测试代码（TDD 红阶段）

**AI 行为**：
- 根据已批准的测试方案，编写**失败的测试代码**。
- 输出测试文件路径及关键代码片段。
- 运行测试确认**确实失败**（RED 状态）。

**产出物**：
- 测试代码文件（例如 `MfaAuthServiceTest.java`）

**门禁 🚪**  
用户检查测试代码合理性后回复 `CONTINUE`，进入 Step 4。

---

## Step 4：实现代码 + 全量回归验证 + 归档

### 目标
实现最小代码使所有测试通过，并执行全量回归验证，最终归档。

> 若在实现时发现规格存在矛盾或无法实现，立即停止编码，记录冲突点，输出修正建议，请求用户决策。可能需要回到 Step 2 修订规格。

### AI 行为（GREEN + REFACTOR + 验证）

1. **实现代码**：编写刚好使 Step 3 测试通过的最简实现。
2. **运行全量测试**：执行 `mvn test` 或对应语言的全量测试命令，确保**所有已有测试**（不仅仅是本次新增）都通过。
    - 若已有测试失败，立即修复（除非是预期变更）。
3. **运行 OpenSpec 验证**：`openspec validate <change-name>`
4. **重构**：优化代码，保持测试通过。
5. **输出验证报告**：

```markdown
## VERIFICATION_REPORT
- 全量测试: ✅ 通过 (125/125)
- OpenSpec 验证: ✅ 通过
- 测试覆盖率: 87%
- 调试代码残留: 无
```

6. **等待用户批准归档**。

### 门禁 🚪
用户验证报告无误后回复 `ARCHIVE`。

### 归档
执行 `openspec archive <change-name>`，将变更移至 `openspec/archive/`，并更新 `openspec/specs/`。

### 最终清理
用户**新建会话**，开始下一个功能。

---

## 改进要点总结

| 原缺陷                    | 优化措施                                                     |
| ------------------------- | ------------------------------------------------------------ |
| Step 1 与 Step 2 职责重叠 | Step 1 输出固定格式 `DESIGN_SUMMARY_FOR_OPENSPEC`，Step 2 直接消费 |
| 只运行新测试，无全量回归  | Step 4 强制运行全量测试，失败必须修复                        |
| 单会话上下文污染          | Step 1+2 一个会话，Step 3+4 新建会话（只加载规格）           |
| 测试方案未充分评审        | 将 Step 3 拆分为“方案评审门禁” → “编写测试代码”              |
| 无规格修订路径            | Step 4 发现规格不可行时，暂停并报告冲突，提供修正选项        |

---

## 用户侧 Prompt 模板（可直接使用）

### 启动 Step 1+2 的对话

```text
**【Step 1: Brainstorming】**  
需求：  
技术栈：Spring Boot 2.7.8 + MyBatis-Plus + Vue 3  
约束：[例：CodeCheck 只支持轮询]  
请使用 brainstorming 技能输出设计摘要（包含 DESIGN_SUMMARY_FOR_OPENSPEC 区块）。  

**【等待我回复 APPROVED 后，继续 Step 2】**  
（我批准后，在同一对话中继续）  
请根据上一步的设计摘要，使用 OpenSpec 创建变更提案（/opsx:ff）。产出 proposal.md、spec.md、design.md、tasks.md。  
```

### 启动 Step 3+4 的新会话

```text
执行“上班打卡”中的 项目上下文加载（读取 PROGRESS.md、DECISIONS.md），但可跳过轻量流程的其余步骤。
**【Step 3: TDD 测试方案】**  
请读取 `openspec/changes/add-mfa/` 下的规格文件，先输出测试方案（使用 TEST_PLAN 格式），等待我回复 TEST_PLAN_APPROVED。  

**【收到 TEST_PLAN_APPROVED 后】**  
现在请根据已批准的测试方案，编写失败的测试代码（TDD 红阶段）。  

**【收到 CONTINUE 后】**  
现在请实现代码并通过所有测试。完成时运行全量测试和 openspec validate，输出验证报告。  

**【收到 ARCHIVE 后】**  
执行 openspec archive。  
```
