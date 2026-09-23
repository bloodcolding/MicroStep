# data-storage Specification

## Purpose
TBD - created by archiving change refactor-to-tauri-v2. Update Purpose after archive.
## Requirements
### Requirement: AppData 落盘

事件流 SHALL 存储于系统应用数据目录（Tauri `app_data_dir()`），不再位于仓库工作目录。所有读写 SHALL 仅指向该目录下的 `events.jsonl`。

#### Scenario: 首次启动初始化

- **WHEN** 全新环境首次启动应用
- **THEN** `app_data_dir` 下创建 `events.jsonl` 并写入初始事件（默认里程碑「无限进步」），应用进入可用状态

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

### Requirement: 旧数据迁移

从 Python 版迁移 SHALL 以文档提供明确的手动路径（将仓库内 `data/events.jsonl` 拷贝至 AppData 数据目录）。应用检测到数据目录已有事件文件时 SHALL NOT 覆盖、清空或重置。

#### Scenario: 带存量数据启动

- **WHEN** 数据目录已存在用户拷入的历史 events.jsonl 并启动应用
- **THEN** 应用重放该文件并继续追加，历史数据完整保留

#### Scenario: 不覆盖保护

- **WHEN** 应用初始化逻辑执行时发现 events.jsonl 已存在
- **THEN** 跳过初始事件写入，保留既有文件内容

### Requirement: 读取容错与缓存等价

事件流读取 SHALL 保持既有语义：损坏的 JSON 行容错跳过且不崩溃（跳过行为可观测）；按文件 mtime 缓存已读内容以避免重复整读，当文件被外部修改（mtime 变化）时缓存 SHALL 自动失效并重新加载。

#### Scenario: 损坏行容错

- **WHEN** 事件文件中混入非法 JSON 行
- **THEN** 该行被跳过且应用不崩溃，其余事件正常重放

#### Scenario: mtime 缓存命中与失效

- **WHEN** 事件文件未变化时连续读取
- **THEN** 使用缓存内容不重复解析全文

- **WHEN** 事件文件被外部程序修改（mtime 变化）后读取
- **THEN** 缓存失效并重新加载全文

