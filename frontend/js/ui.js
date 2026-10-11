// 轻量 UI 反馈：模态框开合（含移动端 History 返回栈与软键盘避让）与 toast 提示。

import { $ } from "./utils.js";

// 覆盖层是否打开：openModal/closeModal 是所有模态（业务表单 + 设置面板）的唯一开合咽喉。
let overlayOpen = false;

export function openModal(title, bodyHtml) {
  $("#modalTitle").textContent = title;
  $("#modalBody").innerHTML = bodyHtml;
  $("#modal").hidden = false;
  // 仅首次打开入栈：语言切换触发的模态重建不重复 push；快速「关→开」时上一轮
  // history.back() 可能尚未结算（栈顶残留 overlay entry），此时改用 replaceState
  // 复用残留项，保证返回栈零增长、不产生幽灵后退。
  if (!overlayOpen) {
    overlayOpen = true;
    if (history.state?.microstepOverlay) {
      history.replaceState({ microstepOverlay: true }, "");
    } else {
      history.pushState({ microstepOverlay: true }, "");
    }
  }
}

export function closeModal(fromPopstate = false) {
  if (!overlayOpen) return;
  $("#modal").hidden = true;
  $("#modalBody").innerHTML = "";
  overlayOpen = false;
  // 用户主动关闭（× / 取消 / 遮罩 / 提交成功）经 history.back() 收敛到同一历史状态；
  // popstate 触发的关闭说明浏览器已消费该历史项，不能再 back。
  if (!fromPopstate && history.state?.microstepOverlay) history.back();
}

// 系统返回（Android 手势 / 返回键）：先关闭最上层覆盖层；无覆盖层时不拦截，退出交还宿主应用。
window.addEventListener("popstate", () => {
  if (overlayOpen) closeModal(true);
  // 追赶被忽略 / 未结算的 back：popstate 落在残留 overlay 历史项上时继续后退，
  // 保证关闭后不遗留「按一次返回无反应」的幽灵项，最终回到宿主退出语义。
  else if (history.state?.microstepOverlay) history.back();
});

// 软键盘避让：visualViewport 可用时把可视高度写入 --app-vh 驱动模态可用高度；
// API 不可用时回退既有 100vh 布局与浏览器默认聚焦滚动，不产生脚本错误。
const viewport = window.visualViewport;

function syncAppViewportHeight() {
  document.documentElement.style.setProperty(
    "--app-vh",
    `${Math.round(viewport ? viewport.height : window.innerHeight)}px`
  );
}

if (viewport) {
  viewport.addEventListener("resize", syncAppViewportHeight);
  viewport.addEventListener("scroll", syncAppViewportHeight);
}
window.addEventListener("resize", syncAppViewportHeight);
window.addEventListener("orientationchange", syncAppViewportHeight);
syncAppViewportHeight();

// 键盘弹出（视口收缩）时把聚焦控件滚入可见区，提交 / 取消路径保持可达。
document.addEventListener("focusin", (event) => {
  if (!overlayOpen) return;
  if (typeof event.target.scrollIntoView === "function") {
    event.target.scrollIntoView({ block: "nearest" });
  }
});

let toastTimer = 0;

export function toast(message, isError = false) {
  const element = $("#toast");
  element.textContent = message;
  element.className = `toast${isError ? " error" : ""}`;
  element.hidden = false;
  window.clearTimeout(toastTimer);
  toastTimer = window.setTimeout(() => {
    element.hidden = true;
  }, 3600);
}
