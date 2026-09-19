// 里程碑列表渲染：过滤 + 排序 + 卡片模板。

import { getState, getEpicSearchQuery } from "../state.js";
import { dimensionName } from "../dimensions.js";
import { escapeHtml, formatPercent } from "../utils.js";

export function renderEpics() {
  const state = getState();
  const query = getEpicSearchQuery();
  const host = document.getElementById("epicList");
  if (!host) return;
  host.innerHTML = "";
  const allEpics = Object.values(state.epics);
  const epics = allEpics
    .filter((epic) => epicMatchesQuery(epic, query))
    .sort((a, b) => {
      if (a.status !== b.status) return a.status === "active" ? -1 : 1;
      return String(b.created_at).localeCompare(String(a.created_at));
    });
  const count = document.getElementById("epicCount");
  if (count) {
    count.textContent = query
      ? `${epics.length} / ${allEpics.length} 个里程碑`
      : `${allEpics.length} 个里程碑`;
  }
  if (!epics.length) {
    host.innerHTML = query
      ? '<p class="muted">没有匹配的里程碑。试试搜索名称、描述、属性或状态。</p>'
      : '<p class="muted">还没有里程碑。创建一个方向，让日常 Task 有处可归。</p>';
    return;
  }
  epics.forEach((epic) => {
    const card = document.createElement("article");
    card.className = `epic-card ${epic.status === "completed" ? "completed" : ""}`;
    const completed = epic.status === "completed";
    const taskCount = epic.task_ids?.length || 0;
    const bonusDimension = epic.title_bonus_dimension || epic.main_dimension;
    card.innerHTML = `
      <div class="epic-top">
        <div>
          <h4>${escapeHtml(epic.title)}</h4>
          <p>${escapeHtml(epic.description || "没有描述")}</p>
        </div>
        <div class="epic-actions">
          <button class="small-button" data-edit-epic="${epic.id}">编辑</button>
          ${
            completed
              ? '<span class="meta-pill">已结项</span>'
              : '<button class="small-button" data-complete-epic="' + epic.id + '">结项祭坛</button>'
          }
        </div>
      </div>
      <div class="epic-meta">
        <span class="meta-pill">${dimensionName(epic.main_dimension)}</span>
        <span class="meta-pill">称号加成：${dimensionName(bonusDimension)} ${formatPercent(epic.title_bonus_percent)}</span>
        <span class="meta-pill">${taskCount} 个 Task</span>
      </div>
      ${
        completed && epic.engraving
          ? `<div class="engraving">“${escapeHtml(epic.engraving)}”</div>`
          : ""
      }
    `;
    host.appendChild(card);
  });
}

function epicMatchesQuery(epic, query) {
  if (!query) return true;
  const haystack = [
    epic.title,
    epic.description,
    epic.main_dimension,
    dimensionName(epic.main_dimension),
    epic.status === "completed" ? "已结项" : "进行中",
    epic.engraving,
  ]
    .join(" ")
    .toLowerCase();
  return haystack.includes(query);
}
