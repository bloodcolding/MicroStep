// DOM 查询与数值/文本格式化工具，无业务状态。

export const $ = (selector) => document.querySelector(selector);
export const $$ = (selector) => Array.from(document.querySelectorAll(selector));

export function clamp(value, min, max) {
  return Math.min(max, Math.max(min, value));
}

export function formatDelta(value) {
  const number = Number(value || 0);
  return `${number >= 0 ? "+" : ""}${number.toLocaleString("zh-CN", { maximumFractionDigits: 1 })}`;
}

export function formatPercent(value) {
  const number = Number(value || 0);
  return `${number >= 0 ? "+" : ""}${number.toLocaleString("zh-CN", { maximumFractionDigits: 2 })}%`;
}

export function formatDateTime(value) {
  if (!value) return "未知时间";
  const date = new Date(value);
  if (Number.isNaN(date.getTime())) return String(value);
  return date.toLocaleString("zh-CN", {
    year: "numeric",
    month: "2-digit",
    day: "2-digit",
    hour: "2-digit",
    minute: "2-digit",
    second: "2-digit",
    hour12: false,
  });
}

export function escapeHtml(value) {
  return String(value ?? "")
    .replaceAll("&", "&amp;")
    .replaceAll("<", "&lt;")
    .replaceAll(">", "&gt;")
    .replaceAll('"', "&quot;")
    .replaceAll("'", "&#039;");
}
