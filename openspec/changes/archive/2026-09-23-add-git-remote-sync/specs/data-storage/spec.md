# data-storage · 规格增量

## MODIFIED Requirements

### Requirement: 数据目录 Git 化

数据目录 SHALL 在初始化时具备 git 仓库结构（手工 `.git` 骨架逻辑保留，幂等、不覆盖既有仓库）。启用远端同步（data-sync）后，同步操作 SHALL 在该仓库写入快照 commit（blob 为 `events.jsonl` 全量快照）。远端配置（URL / PAT / 分支）SHALL 持久化于数据目录内 `sync.json`，SHALL NOT 写入 `.git/config`——数据仓库由应用管理，用户手动 git 操作不在支持范围（灾难恢复走文档化路径：手工 clone 远端后拷贝 `events.jsonl`）。

#### Scenario: git 仓库就绪

- **WHEN** 数据目录初始化完成
- **THEN** 目录内存在 `.git`，后续同步变更可直接提交

#### Scenario: 同步写入快照 commit

- **WHEN** 同步完成且 `events.jsonl` 相对上次 commit 有变化
- **THEN** 数据目录 git 仓库新增快照 commit，仅同步动作产生 commit

#### Scenario: 应用管理边界

- **WHEN** 配置远端并完成同步
- **THEN** `.git/config` 不含 remote 配置，远端信息仅存在于 `sync.json`
