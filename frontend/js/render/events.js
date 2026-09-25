// 事件流渲染：简单视图与仪表盘共用，按创建时间倒序截取。

import { t, hasTranslation } from "../i18n/index.js";
import { getState } from "../state.js";
import { dimensionName } from "../dimensions.js";
import { formatDelta, formatDateTime, escapeHtml } from "../utils.js";

export function renderEvents(containerId, limit = 10) {
  const state = getState();
  const host = document.getElementById(containerId);
  if (!host) return;
  host.innerHTML = "";
  const events = (state.event_stream || [])
    .slice()
    .sort((a, b) => String(b.created_at || "").localeCompare(String(a.created_at || "")))
    .slice(0, limit);
  if (!events.length) {
    host.innerHTML = `<p class="muted">${t("events.empty")}</p>`;
    return;
  }
  events.forEach((event) => {
    host.appendChild(renderEventItem(event));
  });
}

export function renderEventItem(event) {
  const item = document.createElement("div");
  const deleted = Boolean(event.deleted);
  item.className = `event-item ${deleted ? "deleted" : ""}`;
  const eventId = event.event_id;
  const changes = Array.isArray(event.changes) ? event.changes : [];
  const changesHtml = changes.length
    ? changes
        .map(
          (change) =>
            `<span class="effect-chip">${dimensionName(change.dimension)} ${formatDelta(change.delta)}</span>`
        )
        .join("")
    : `<span class="muted">${t("events.noChanges")}</span>`;
  const detailTags = event.details?.tags?.length
    ? `<p class="event-detail">${escapeHtml(event.details.tags.join(" / "))}</p>`
    : "";
  const engraving = event.details?.engraving
    ? `<p class="event-detail">“${escapeHtml(event.details.engraving)}”</p>`
    : "";
  const deleteButton =
    eventId && !deleted
      ? `<button class="icon-button" type="button" data-delete-event="${escapeHtml(eventId)}" title="${t("events.markDeleted")}">🗑</button>`
      : "";
  const icon = deleted ? "🗑️" : eventIcon(event.type);
  // 类型 chip 是前端生成描述（走字典）；条目标题 event.name 为后端生成（含用户数据），按翻译边界原文直显。
  const typeKey = `event.type.${event.type}`;
  const typeLabel = deleted
    ? t("events.typeDeleted")
    : hasTranslation(typeKey)
      ? t(typeKey)
      : event.type;
  item.innerHTML = `
    <div class="event-icon ${deleted ? "deleted-icon" : ""}">${icon}</div>
    <div class="event-main">
      <div class="event-title-row">
        <strong>${escapeHtml(event.name || event.type)}</strong>
        <span class="event-type ${deleted ? "deleted-type" : ""}">${escapeHtml(typeLabel || "")}</span>
      </div>
      <div class="event-changes">${changesHtml}</div>
      ${detailTags}
      ${engraving}
      <p class="event-time">${t("events.createdAt", { time: formatDateTime(event.created_at) })}${deleted ? t("events.deletedSuffix", { time: formatDateTime(event.deleted_at) }) : ""}</p>
    </div>
    <div class="event-actions">${deleteButton}</div>
  `;
  return item;
}

function eventIcon(type) {
  return {
    TASK_CREATED: "🧩",
    TASK_COMPLETED: "✅",
    TASK_UPDATED: "✏️",
    TASK_DELETED: "🗑️",
    EPIC_CREATED: "🎯",
    EPIC_UPDATED: "✏️",
    EPIC_COMPLETED: "🏆",
    ACTION_LOGGED: "📜",
    TITLE_EQUIPPED: "🏅",
    TITLE_UNEQUIPPED: "↩️",
    PROFILE_AWAKENED: "🌱",
    SYSTEM_DAILY_TICK: "⏱️",
    SYSTEM_INIT: "⚙️",
  }[type] || "•";
}
