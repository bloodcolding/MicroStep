// 轻量 UI 反馈：模态框开合与 toast 提示。

import { $ } from "./utils.js";

export function openModal(title, bodyHtml) {
  $("#modalTitle").textContent = title;
  $("#modalBody").innerHTML = bodyHtml;
  $("#modal").hidden = false;
}

export function closeModal() {
  $("#modal").hidden = true;
  $("#modalBody").innerHTML = "";
}

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
