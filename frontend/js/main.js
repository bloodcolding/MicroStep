// 应用入口：初始化日期标签、绑定全局事件（列表交互用事件委托，渲染层不反向依赖 actions）。

import { initI18n, getLocale, toggleLocale, onLocaleChanged, currentNumberLocale } from "./i18n/index.js";
import { $ } from "./utils.js";
import {
  setTaskSearchQuery,
  setEpicSearchQuery,
  isShowCompletedTasks,
  setShowCompletedTasks,
} from "./state.js";
import { renderTasks } from "./render/tasks.js";
import { renderEpics } from "./render/epics.js";
import { loadMeta, loadState, renderAll } from "./controller.js";
import * as actions from "./actions.js";
import * as modals from "./modals.js";
import { closeModal, toast } from "./ui.js";

document.addEventListener("DOMContentLoaded", () => {
  initI18n();
  updateTodayLabel();
  updateLocaleToggleLabel();
  bindEvents();
  loadMeta().then(loadState).catch((error) => toast(error.message, true));
});

// 今日日期标签：格式化 locale 跟随界面语言。
function updateTodayLabel() {
  $("#todayLabel").textContent = new Date().toLocaleDateString(currentNumberLocale(), {
    year: "numeric",
    month: "long",
    day: "numeric",
    weekday: "short",
  });
}

// 切换按钮显示另一语言的名称（zh 界面显示 EN，en 界面显示「中文」）。
function updateLocaleToggleLabel() {
  $("#localeToggle").textContent = getLocale() === "zh-CN" ? "EN" : "中文";
}

function bindEvents() {
  $("#localeToggle").addEventListener("click", () => {
    toggleLocale();
    updateLocaleToggleLabel();
  });
  // 语言切换：静态文案由 setLocale 内部重放，这里补日期/按钮态并驱动全量重渲染。
  onLocaleChanged(() => {
    updateTodayLabel();
    updateLocaleToggleLabel();
    renderAll();
  });
  $("#awakenTopBtn").addEventListener("click", actions.awaken);
  $("#refreshBtn").addEventListener("click", () => loadState().catch((error) => toast(error.message, true)));
  $("#newEpicBtn").addEventListener("click", () => modals.openEpicModal());
  $("#newTaskBtn").addEventListener("click", () => modals.openTaskModal());
  $("#simpleNewTaskBtn").addEventListener("click", () => modals.openTaskModal());
  $("#taskSearchInput").addEventListener("input", (event) => {
    setTaskSearchQuery(event.target.value.trim().toLowerCase());
    renderTasks();
  });
  $("#epicSearchInput").addEventListener("input", (event) => {
    setEpicSearchQuery(event.target.value.trim().toLowerCase());
    renderEpics();
  });
  const showCompletedToggle = $("#showCompletedTasks");
  showCompletedToggle.checked = isShowCompletedTasks();
  showCompletedToggle.addEventListener("change", (event) => {
    setShowCompletedTasks(event.target.checked);
    renderTasks();
  });
  $("#modal").addEventListener("click", (event) => {
    if (event.target.matches("[data-close-modal]")) closeModal();
  });
  $("#modalBody").addEventListener("submit", modals.handleModalSubmit);

  // 列表内容每次重建，交互统一委托到容器，避免渲染后重复绑定。
  delegate("taskList", "data-complete-task", actions.completeTask);
  delegate("taskList", "data-edit-task", modals.openTaskModal);
  delegate("taskList", "data-delete-task", actions.deleteTask);
  delegate("epicList", "data-complete-epic", modals.openCompleteEpicModal);
  delegate("epicList", "data-edit-epic", modals.openEpicModal);
  delegate("titleLibrary", "data-equip-title", actions.equipTitle);
  delegate("titleLibrary", "data-unequip-title", actions.unequipTitle);
  delegate("equipmentSlots", "data-unequip-slot", actions.unequipTitle);
  delegate("simpleEvents", "data-delete-event", actions.deleteEvent);
  delegate("dashboardEvents", "data-delete-event", actions.deleteEvent);
}

// 在容器上代理 data-* 属性点击，命中后把属性值交给 handler。
function delegate(containerId, attributeName, handler) {
  const container = document.getElementById(containerId);
  if (!container) return;
  container.addEventListener("click", (event) => {
    const target = event.target.closest(`[${attributeName}]`);
    if (!target || !container.contains(target)) return;
    handler(target.getAttribute(attributeName));
  });
}
