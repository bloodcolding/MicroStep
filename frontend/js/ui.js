// 轻量 UI 反馈：模态框开合、覆盖层返回栈、软键盘避让与 toast 提示。

import { $ } from "./utils.js";

// 覆盖层返回栈：打开时 pushState，系统返回（popstate）先关模态；无覆盖层时不拦截宿主退出。
let overlayOpen = false;
let ignoreNextPopstate = false;

export function openModal(title, bodyHtml) {
  $("#modalTitle").textContent = title;
  $("#modalBody").innerHTML = bodyHtml;
  $("#modal").hidden = false;
  // 语言切换等场景会按原参数重建已打开的模态：不重复压栈，避免幽灵返回。
  if (!overlayOpen) {
    overlayOpen = true;
    history.pushState({ microstepOverlay: true }, "");
  }
}

export function closeModal() {
  $("#modal").hidden = true;
  $("#modalBody").innerHTML = "";
  $("#modal .modal-card").classList.remove("settings-modal");
  if (overlayOpen) {
    overlayOpen = false;
    // 常规关闭路径（× / 取消 / 遮罩 / 提交成功）：主动弹出本次压入的 History entry。
    if (history.state && history.state.microstepOverlay) {
      ignoreNextPopstate = true;
      history.back();
    }
  }
}

window.addEventListener("popstate", () => {
  if (ignoreNextPopstate) {
    ignoreNextPopstate = false;
    return;
  }
  if (overlayOpen && !$("#modal").hidden) {
    overlayOpen = false;
    $("#modal").hidden = true;
    $("#modalBody").innerHTML = "";
    $("#modal .modal-card").classList.remove("settings-modal");
  }
});

// 软键盘避让：visualViewport 高度同步为 CSS 变量，小屏模态按动态视口收缩；API 缺失时静默回退静态视口。
if (window.visualViewport) {
  const syncVisualHeight = () => {
    document.documentElement.style.setProperty("--ms-visual-vh", `${Math.round(window.visualViewport.height)}px`);
  };
  window.visualViewport.addEventListener("resize", syncVisualHeight);
  syncVisualHeight();
}

// 聚焦模态内输入控件时滚入可见区，保证软键盘弹出后聚焦控件与提交 / 取消路径可达。
document.addEventListener("focusin", (event) => {
  const target = event.target;
  if (target instanceof HTMLElement && target.closest("#modalBody")) {
    window.requestAnimationFrame(() => target.scrollIntoView({ block: "nearest" }));
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