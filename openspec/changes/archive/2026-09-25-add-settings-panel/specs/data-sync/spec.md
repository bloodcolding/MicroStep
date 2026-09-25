# data-sync · 规格增量

## MODIFIED Requirements

### Requirement: 同步设置界面

前端 SHALL 在设置面板「代码仓同步」分类（见 settings-ui 能力）中提供远端 URL / PAT / 分支的配置表单（保存调 `sync_set_config`）、「立即同步」按钮（点击前自动保存表单，保存失败中止；调 `sync_now`）、最近同步时间与结果展示。PAT 输入框 SHALL 以密码形态展示，留空保存 SHALL 保持既有 PAT 不变，勾选「清除已保存 PAT」SHALL 发送空串清除。调用链 SHALL 经 `api.js` 唯一收口，IPC command 契约零变化。`frontend/js/sync.js` SHALL 以模块导出（表单片段 + 绑定 + 回显刷新）形式被设置面板挂载，SHALL NOT 再自绑定仪表盘挂载点；仪表盘 SHALL NOT 保留同步卡片（入口唯一化见 settings-ui）。

#### Scenario: 配置并同步

- **WHEN** 用户在设置面板「代码仓同步」分类填写远端信息保存后点击「立即同步」
- **THEN** 调用链经 api.js 收口完成，面板显示统计结果并刷新最近同步状态

#### Scenario: PAT 密码形态与保持语义

- **WHEN** PAT 框以密码形态展示，且用户留空保存 / 勾选清除 / 输入新值保存
- **THEN** 分别保持既有 PAT、清除 PAT、更新 PAT，回显仅显示脱敏形态

#### Scenario: 模块挂载方式

- **WHEN** 审计 `frontend/js/sync.js` 与 `settings.js`
- **THEN** sync.js 导出表单片段与行为函数，由 settings.js 挂载至设置面板同步分类；index.html 不再单独引入 sync.js 作为自绑定入口
