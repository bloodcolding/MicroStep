// Task 列表渲染：过滤（搜索词/显示已完成）+ 状态排序 + 行模板。

import { t } from "../i18n/index.js";
import { getState, getTaskSearchQuery, isShowCompletedTasks } from "../state.js";
import { dimensionName, taskEffects } from "../dimensions.js";
import { escapeHtml, formatDelta } from "../utils.js";

export function renderTasks() {
  const state = getState();
  const query = getTaskSearchQuery();
  const showCompleted = isShowCompletedTasks();
  const host = document.getElementById("taskList");
  if (!host) return;
  host.innerHTML = "";
  const allTasks = Object.values(state.tasks).filter(
    (task) => !task.deleted && task.status !== "deleted"
  );
  const tasks = allTasks
    .filter((task) => {
      if (!showCompleted && task.status === "completed") return false;
      return taskMatchesQuery(task, query);
    })
    .sort((a, b) => {
      if (a.status !== b.status) return a.status === "active" ? -1 : 1;
      return String(b.created_at).localeCompare(String(a.created_at));
    });
  const count = document.getElementById("taskCount");
  if (count) {
    count.textContent = query || !showCompleted
      ? t("task.count.some", { shown: tasks.length, total: allTasks.length, count: tasks.length })
      : t("task.count.all", { count: allTasks.length });
  }
  if (!tasks.length) {
    if (!showCompleted && !query) {
      host.innerHTML = `<p class="muted">${t("task.empty.active")}</p>`;
    } else if (query) {
      host.innerHTML = `<p class="muted">${t("task.empty.search")}</p>`;
    } else {
      host.innerHTML = `<p class="muted">${t("task.empty.none")}</p>`;
    }
    return;
  }
  tasks.forEach((task) => {
    const epic = state.epics[task.epic_id];
    const effects = taskEffects(task);
    const completed = task.status === "completed";
    const repeatable = Boolean(task.repeatable);
    const times = Number(task.times_completed || 0);
    const sanDelta = effects
      .filter((effect) => effect.dimension === "san")
      .reduce((total, effect) => total + Number(effect.delta || 0), 0);
    const sanInsufficient = sanDelta < 0 && Number(state.derived.san) + sanDelta < 0;
    const row = document.createElement("div");
    row.className = `task-row ${completed ? "completed" : ""}`;
    row.innerHTML = `
      <button class="task-check" data-complete-task="${task.id}" ${(completed && !repeatable) || sanInsufficient ? "disabled" : ""} title="${sanInsufficient ? t("task.sanNeedTitle", { need: Math.abs(sanDelta) }) : ""}">${repeatable ? "＋" : "✓"}</button>
      <div>
        <strong>${escapeHtml(task.title)}</strong>
        <p class="muted">
          ${escapeHtml(epic?.title || t("task.independent"))}
          ${repeatable ? ` · ${t("task.repeatable")}${times ? t("task.timesCompleted", { count: times }) : ""}` : ""}
        </p>
      </div>
      <span class="task-value">
        ${effects.map((effect) => `<span class="effect-chip">${dimensionName(effect.dimension)} ${formatDelta(effect.delta)}</span>`).join("")}
        ${sanInsufficient ? `<span class="warning-chip">${t("task.sanInsufficient")}</span>` : ""}
      </span>
      <div class="task-actions">
        <button class="small-button" type="button" data-edit-task="${task.id}">${t("common.edit")}</button>
        <button class="icon-button" type="button" data-delete-task="${task.id}" title="${t("task.deleteTitle")}">🗑</button>
      </div>
    `;
    host.appendChild(row);
  });
}

function taskMatchesQuery(task, query) {
  if (!query) return true;
  const state = getState();
  const epic = state.epics[task.epic_id];
  const effects = taskEffects(task)
    .map((effect) => `${effect.dimension} ${dimensionName(effect.dimension)} ${effect.delta} ${formatDelta(effect.delta)}`)
    .join(" ");
  const haystack = [
    task.title,
    epic?.title || t("task.independent"),
    effects,
    task.repeatable ? t("task.searchRepeatable") : t("task.searchOnce"),
    task.status === "completed" ? t("task.searchCompleted") : t("task.searchActive"),
  ]
    .join(" ")
    .toLowerCase();
  return haystack.includes(query);
}
