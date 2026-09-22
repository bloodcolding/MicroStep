# tasks.md · add-git-remote-sync

> 初始占位版本（Step 2 产出）。Step 3 TDD 时将按测试方案细化为可勾选执行清单。

## 1. 依赖与 ADR

- [ ] 1.1 gix 最小 feature 集引入（open/fetch/blob 读写/push/http transport），`cargo build` 零警告，Cargo.lock 锁定，记录依赖树实测大小
- [ ] 1.2 追加 ADR-004（gix 引入 + ADR-002 白名单扩展），回填 design.md Q1 可行性核查结论

## 2. 同步配置（SyncConfig）

- [ ] 2.1 `sync.json` 读写：字段定义、缺省值、部分更新语义（未携带字段不变、pat 空串清除）
- [ ] 2.2 IPC：`sync_get_config`（PAT 脱敏末 4 位）/ `sync_set_config` 信封方法与 command（camelCase 参数键）

## 3. Union Merge 核心（纯函数）

- [ ] 3.1 合并算法实现：本地序保留 + 远端独有追加 + 语义相等去重 + 同 id 冲突拒绝 + 无 id 行处理
- [ ] 3.2 单测全分支 + 确定性断言（同输入同输出字节）

## 4. Git 传输（SyncEngine）

- [ ] 4.1 fetch：远端分支 tip + events.jsonl blob 读取（无 checkout）
- [ ] 4.2 对象写入：blob/tree/commit（双 parent 合流），快照式（仅同步时 commit）
- [ ] 4.3 push：non-fast-forward 有界重试（≤2）+ 超时（30s）+ 错误分类映射
- [ ] 4.4 集成测试：本地 bare repo 充当远端——推种子 / 双向同步 / 并发推进重试 / bootstrap 三场景

## 5. 触发链与并发

- [ ] 5.1 启动 best-effort pull：setup 阶段异步 spawn，失败记 last_result，不阻塞不弹窗
- [ ] 5.2 `sync_now`：完整 pull-merge-push + {pulled, pushed, merged} 统计 + last_sync_at/last_result 更新
- [ ] 5.3 持锁语义：同步期间业务写阻塞等待的并发测试

## 6. 前端同步面板

- [ ] 6.1 `frontend/js/sync.js`：配置表单（URL/PAT 密码形态/分支）+ 同步按钮 + 最近结果展示，经 api.js 收口
- [ ] 6.2 `index.html` 挂载入口；其余 17 个前端模块零改动（diff 证明）

## 7. 验收与收尾

- [ ] 7.1 全量回归：cargo test 全绿（既有 79 例 + 新增），无新增警告
- [ ] 7.2 真实远端手工冒烟：GitHub + Gitee 各至少一次完整双向同步（含错误路径：错 PAT）
- [ ] 7.3 文档：README 同步配置节 + 产品方案/前端设计补段 + PROGRESS.md 归档总结
