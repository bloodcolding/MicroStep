# verification.md · add-mobile-ui-adaptation

> Step 3 红灯基线记录（2026-09-29）。测试计划：TC-M01~M28（会话 TEST_PLAN，已获 TEST_PLAN_APPROVED）。
> 基线环境：worktree `7a8d\MicroStep` @ `3bbca1a`（零代码改动）；WebView2 `Edg/154.0.4258.37`（CDP :9222 + `Emulation.setDeviceMetricsOverride`）；Node v24.14 一次性驱动脚本位于 `%TEMP%`（不入仓库）；被测应用为 `cargo build`（dev，前端资产内嵌）。冒烟仅打开/关闭模态与可逆视图显隐测量（测后还原），未提交任何表单、未切换语言、未触发业务写操作。

## 1 · L1 静态审计

| TC | 结果 | 证据 |
|---|---|---|
| TC-M01 | 🔴 FAIL | viewport 仅 `width=device-width, initial-scale=1.0`（`index.html:5`），缺 `viewport-fit=cover`；未使用 `user-scalable=no` ✅ |
| TC-M02 | 🔴 FAIL | `styles.css` 零 `env(safe-area-inset-*)`；`overscroll-behavior: contain` 仅在 `.epic-list`(:601)/`.task-list`(:672) 内部列表，根页面无橡皮筋抑制 |
| TC-M09 | 🟢 护栏 | 现仅 2 个 `@media`（1040px / 680px）；实现新增紧凑规则必须限定断点内 |
| TC-M11 | 🔴 FAIL | 零 `touch-action: manipulation`、零 `:active`、零 `:focus-visible`；8 处 `:hover` 承担交互态 |
| TC-M12 | 🟢 现状达标 | 模态内 input/select computed `16px`、`user-select: auto`（L2 实测）；textarea 留绿阶段用 epic 模态复验 |
| TC-M14 | 🔴 FAIL | 全前端零 `history.pushState` / `popstate`（`dimensions.js:60` 的 `history` 是维度历史数组，非 `window.history`） |
| TC-M19 | 🔴 FAIL | 零 `visualViewport` 引用 |
| TC-M22 | 🔴 FAIL | 零 `ResizeObserver` 引用 |
| TC-M25 | 🟢 护栏 | `frontend/` 无 package.json / node_modules / npm 引用 / 打包器配置 |
| TC-M26 | 🟢 护栏 | 基线 `git diff`（除规格同步 untracked）为零改动 |
| TC-M27 | 🟢 护栏 | zh-CN/en 基键集合 177=177（剥 `.one/.other` 复数后）；159 处 `data-i18n*` / `t()` 引用全部解析（en 多出的 `.one` 键为 i18n 既有复数设计，首版粗审为假阳性，已修正口径） |

## 2 · L2 WebView CDP 冒烟（真实 AppData 数据，只读渲染）

### 2.1 视口矩阵

| 视口 | 横向滚动 | dashboard 列 | 命中区<44px 控件 | touch-action |
|---|---|---|---|---|
| 390×844 @3x | 无 ✅ | 单列 ✅ | 30/33 🔴 | auto 🔴 |
| 740×360 @3x | 无 ✅ | 单列 ✅ | 33/33 🔴 | auto 🔴 |
| 960×640 @1x | 无 ✅ | 单列（既有 1040px 断点行为·护栏快照） | 33/33 | auto |
| 1280×860 @1x | 无 ✅ | 双列 739px/466px ✅（护栏快照） | 33/33 | auto |

**超预期发现**：现状 390px 已无横向滚动且双栏已折叠（既有 1040px 断点兜住）。TC-M03 / M05 / M06 的“无横滚 + 单列”现状达标，**转为防回归护栏**；移动适配红灯集中在触控 / 覆盖层返回栈 / 键盘 / Canvas / 安全区。

### 2.2 逐项判定

- **TC-M03** 🟢 现状达标：390px 无横滚，顶栏按钮可见（awakenTopBtn 为已觉醒业务态隐藏，非布局问题）。
- **TC-M05** 🟢 现状达标：simpleView 强制显隐测量（测后还原）：hero 366px / SAN 条 / 新建 Task / 事件流均 ≤390px，单列无横滚。
- **TC-M06** 🟢 现状达标：dashboard 390px 单列（`366.174px`），可纵向浏览。
- **TC-M07** 🔴 部分：740×360 无横滚 ✅、主操作可见 ✅，但“顶栏与卡片高度压缩”零规则。
- **TC-M08** 🟢 护栏快照：960=单列、1280=双列 + 居中模态（task topGap=bottomGap=22，settings 87）；实现后 computed 需与快照一致。
- **TC-M10** 🔴：390px 下 30/33 可见控件命中区 <44px（localeToggle 52×40、newEpicBtn 321×30、编辑 53×36、任务 ＋ 24×24 等）。
- **TC-M13** 🔴：`.primary-button` computed `touch-action: auto` → 双击缩放风险（viewport 未禁缩放本身 ✅）。
- **TC-M15** 🔴：模态开启时派发 `popstate` → 模态仍开启（wasOpen=true, stillOpen=true）。
- **TC-M17** 🔴：390px 模态为居中卡片（task：topGap=bottomGap=20、w=350、radius=22），非 bottom sheet；`max-height: 100vh−40`（静态视口，非动态视口）；1280px 居中形态 ✅ 护栏。
- **TC-M18** 🔴：依赖 M14（零 history 接线）；未实际切语言以免触碰 localStorage。
- **TC-M21** 🔴：零 `visualViewport` 逻辑，键盘收缩行为无法演示（结构性缺失；WebView2 本身提供该 API）。
- **TC-M23** 🔴：canvas CSS 321×284 @dpr3，backing store 1727×1528（≈初始加载时 575px 卡宽 ×3，ratioOk=false）→ 未按当前容器尺寸绘制。
- **TC-M24** 🔴：旋转至 740×360 后 CSS 655×580，backing store 仍冻结 1727×1528（无重绘）。

## 3 · L3 Android 真机（待补）

本会话环境无 adb（`Get-Command adb` 失败；常见 Android SDK 路径无命中）。TC-M04 / M13 / M16 / M20 / M24 的真机红灯由 L1/L2 结构性证据推导成立（无 safe-area / 无返回栈 / 无键盘避让 / Canvas 不重绘）；真机绿灯验收留待 adb 环境就绪或用户侧执行后补录本节。

## 4 · L4 全量回归护栏

`cargo test`（沙箱外）：14 suites，**127 passed / 0 failed** ✅。
备注：沙箱内 `tc_e01_unreachable_remote_network_error` 会因环境限制被错误归类为 Auth 而误报失败；沙箱外单例复跑与全量均通过，属环境差异，与本变更无关。

## 5 · 红灯基线结论

- **确立红灯（15）**：TC-M01、M02、M07、M10、M11、M13、M14、M15、M17、M18、M19、M21、M22、M23、M24
- **现状达标转防回归护栏（10）**：TC-M03、M05、M06、M08、M09、M12、M25、M26、M27、M28
- **待真机补录**：TC-M04、M16、M20 + M13/M24 的真机部分

→ 红灯基线成立。等待 `CONTINUE` 进入 Step 4 实现。

## 6 · Step 4 实现与绿灯验证（2026-09-29）

**实现范围（恰 6 文件，全部在 proposal Impact 清单内）**：`index.html`（viewport-fit）、`styles.css`（根安全区 / 触控 / ≤760px 断点 / bottom sheet / 横屏压缩）、`js/ui.js`（覆盖层返回栈 + visualViewport 同步 + focusin 滚动）、`js/render/radar.js`（自适应半径字号 + ResizeObserver + rAF 防同帧反馈）、`js/main.js`（observeRadarResize 接线）、`前端设计.md`（第 9 章）。`api.js` / modals.js / settings.js / i18n 字典 / Rust / Cargo.lock / 原生工程零改动。

### 6.1 L1 复审 → 全绿

TC-M01（viewport-fit=cover ✓ 无禁缩放）、M02（safe-area top/bottom + 0 缺省 + 根 overscroll ✓）、M11（touch-action/:active/:focus-visible ✓）、M14（pushState/popstate 接线 ✓）、M19（visualViewport 存在性守卫 ✓）、M22（ResizeObserver ✓）、M25（零依赖 ✓）、M26（git status 恰 6 文件 ✓）、M27（零新增 key，字典未动 ✓）。

### 6.2 L2 绿灯（WebView2 CDP，新构建）

| 检查 | 结果 |
|---|---|
| 390×844@3x | 无横滚；单列；**below44 = 0/33**（基线 30/33）；touch=manipulation；cardPad 16px |
| 740×360@3x | 无横滚；**below44 = 0/33**；topbar 63px / card 12px（横屏压缩生效，基线 topbar ~83px） |
| 960×640 / 1280×860 | cols/cardPad/touch/below44 **与红灯基线快照逐项一致**（905.216px 单列；739.503px+465.617px 双列；cardPad 22px；touch=auto）→ TC-M08 护栏 ✅ |
| simpleView 390 | 无横滚（强制显隐测量后还原） |
| 模态 390 | 贴底（bottomGap=0）、w=390 满视口、radius 22/22/0/0、maxHeight 820px（动态视口）、margin-bottom≈0；输入 16px、user-select auto → TC-M17 ✅ |
| 模态 1280 | task 居中 22/22、w=560、maxHeight 820.465（与基线一致）；settings 居中 760 → 桌面形态保留 ✅ |
| History 返回栈 | open→pushState ✓；popstate→关闭 ✓；×→弹栈收敛 ✓；连续开关无幽灵 ✓；语言切换重建不重复压栈（TC-M18）✓ |
| 键盘模拟 | vv.height→400：var=400px、maxHeight 376px、margin-bottom 444px、卡片底边=400（抬至可见区）；恢复 844/820 ✓ |
| 雷达 | 390@3x：backing 999×885 = css×dpr，labelsFit ✓；旋转 740：backing 2073×1836 同步重绘，labelsFit ✓（基线：冻结 1727×1528） |
| 无 visualViewport 回退 | API 删除后 reload：0 脚本错误、`--ms-visual-vh` 未注入、maxHeight 回退 100dvh−24=820.444、bottom sheet + 返回栈仍工作 ✓ |
| console | 主轮与回退轮均 0 未捕获错误、0 次 `AGENT-I18N` 警告（ResizeObserver loop 告警已由 rAF 延迟消除） |

**测试脚手架经验**：CDP `Page.reload` 偶发破坏 Tauri IPC 初始化（state 为 null 连锁报错），主轮改为 attach 后免 reload 注入捕获；截图存于 `%TEMP%\ms-green-*.png`（含真实数据，不入库）。

### 6.3 L4 回归

`cargo test`：14 suites **127 passed / 0 failed** ✅；`cargo build`：**0 警告** ✅。

### 6.4 结论

**L1 / L2 / L4 全绿**；L3 Android 真机 5 项（TC-M04 / 13 / 16 / 20 / 24 真机部分）仍待 adb 环境或用户侧执行后补录第 3 节。等待 `ARCHIVE`。