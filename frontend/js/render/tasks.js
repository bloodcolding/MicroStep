// Task 列表渲染：过滤（搜索词/显示已完成）+ 状态排序 + 行模板。

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
      ? `${tasks.length} / ${allTasks.length} 个 Task`
      : `${allTasks.length} 个 Task`;
  }
  if (!tasks.length) {
    if (!showCompleted && !query) {
      host.innerHTML = '<p class="muted">当前没有未完成的 Task。勾选「显示已完成」可以查看全部记录。</p>';
    } else if (query) {
      host.innerHTML = '<p class="muted">没有匹配的 Task。试试搜索名称、属性、数值或里程碑。</p>';
    } else {
      host.innerHTML = '<p class="muted">暂无 Task。点击「新建 Task」，可同时选择多个属性并设置增益或减益。</p>';
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
      <button class="task-check" data-complete-task="${task.id}" ${(completed && !repeatable) || sanInsufficient ? "disabled" : ""} title="${sanInsufficient ? `SAN 不足：需要 ${Math.abs(sanDelta)}` : ""}">${repeatable ? "＋" : "✓"}</button>
      <div>
        <strong>${escapeHtml(task.title)}</strong>
        <p class="muted">
          ${escapeHtml(epic?.title || "独立 Task")}
          ${repeatable ? ` · 可重复${times ? `（已记录 ${times} 次）` : ""}` : ""}
        </p>
      </div>
      <span class="task-value">
        ${effects.map((effect) => `<span class="effect-chip">${dimensionName(effect.dimension)} ${formatDelta(effect.delta)}</span>`).join("")}
        ${sanInsufficient ? '<span class="warning-chip">SAN 不足</span>' : ""}
      </span>
      <div class="task-actions">
        <button class="small-button" type="button" data-edit-task="${task.id}">编辑</button>
        <button class="icon-button" type="button" data-delete-task="${task.id}" title="删除 Task">🗑</button>
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
    epic?.title || "独立 Task",
    effects,
    task.repeatable ? "可重复" : "一次性",
    task.status === "completed" ? "已完成" : "进行中",
  ]
    .join(" ")
    .toLowerCase();
  return haystack.includes(query);
}
