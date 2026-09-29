# tasks.md · add-mobile-ui-adaptation

> Step 2 初始执行清单；Step 3 批准测试方案后可按 TDD / 冒烟策略补充验收细节。

## 1. 设计与文档准备

- [ ] 1.1 按 `mobile-ui` 规格更新 `前端设计.md`，补充移动断点、safe area、模态 bottom sheet、返回栈、键盘避让与雷达图自适应交互
- [ ] 1.2 静态盘点现有控件 / 模态 / Canvas 尺寸路径，列出需要触碰的选择器与模块，确认桌面路径隔离

## 2. 视口与小屏布局

- [ ] 2.1 修改 `index.html` viewport 并加入移动端安全区 / 滚动基础样式
- [ ] 2.2 调整顶栏、hero、Dashboard 卡片、Task / 里程碑 / 事件列表在移动断点的布局与间距，消除横向滚动
- [ ] 2.3 增加手机横屏与 960px+ 桌面防回归样式边界

## 3. 触控与覆盖层体验

- [ ] 3.1 为移动端主要控件补足 44px 命中区、active / focus-visible 反馈、输入可选中和双击不缩放规则
- [ ] 3.2 将小屏模态调整为 bottom sheet / 近全屏形态并预留 safe area，保留桌面居中形态
- [ ] 3.3 用标准 History API 收敛模态打开 / 关闭与 Android 返回栈，覆盖设置面板和业务表单
- [ ] 3.4 引入 `visualViewport` / 等效键盘避让逻辑，保证聚焦输入与提交 / 取消路径可达

## 4. Canvas 自适应

- [ ] 4.1 调整雷达图绘制尺寸计算与标签边距，支持移动容器宽度与 devicePixelRatio
- [ ] 4.2 用 `ResizeObserver` 或等效标准机制监听容器 / 方向变化并复用既有渲染入口

## 5. i18n 与工程边界审计

- [ ] 5.1 为新增用户可见文案 / aria-label 同步补齐 `zh-CN` / `en` 字典
- [ ] 5.2 审计零构建零依赖、IPC / Rust / schema / 原生权限零改动，以及 `api.js` 调用链不变

## 6. 验证与回归

- [ ] 6.1 按批准后的测试计划执行静态审计与响应式 WebView / CDP 冒烟（390×844、740×360、960×640、1280×860）
- [ ] 6.2 Android 真机冒烟：safe area / 触控 / 模态 / 系统返回 / 软键盘 / 旋转雷达图 / 两语言切换
- [ ] 6.3 `cargo test` 全量回归通过；如实现阶段重建前端资产，执行 `cargo build` 确认无新增警告
- [ ] 6.4 更新 `docs/harness/PROGRESS.md`，记录验证结果与遗留风险
