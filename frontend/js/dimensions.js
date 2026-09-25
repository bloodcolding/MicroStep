// 维度取值、预览与 Task 效果读取：连接 state/meta 与各渲染模块。

import { t, dictValue, currentNumberLocale } from "./i18n/index.js";
import { getState, getMeta } from "./state.js";
import { clamp, formatDelta } from "./utils.js";

export function dimensionName(key) {
  return dictValue(`dimension.${key}.name`) || getMeta()?.dimensions?.[key]?.name || key;
}

// 属性卡副标题：字典优先，meta 下发 full_name 兜底。
export function dimensionFullName(key) {
  return dictValue(`dimension.${key}.fullName`) || getMeta()?.dimensions?.[key]?.full_name || "";
}

export function attributeCurrentValue(dimension) {
  const state = getState();
  if (dimension === "san") return Number(state.dimensions.san || 0);
  return Number(state.dimensions[dimension] || 0);
}

export function attributeValueLabel(dimension) {
  const value = attributeCurrentValue(dimension);
  if (dimension === "san") return `${value} / 100`;
  return value.toLocaleString(currentNumberLocale(), { maximumFractionDigits: 1 });
}

export function projectedAttributeLabel(dimension, delta) {
  const value = attributeCurrentValue(dimension);
  const projected = dimension === "san"
    ? clamp(value + delta, 0, 100)
    : Math.max(0, value + delta);
  const unit = dimension === "san" ? " / 100" : "";
  return `→ ${projected.toLocaleString(currentNumberLocale(), { maximumFractionDigits: 1 })}${unit}`;
}

export function taskEffects(task) {
  if (Array.isArray(task.effects) && task.effects.length) return task.effects;
  const effects = [];
  const legacyExp = Number(task.exp_delta ?? task.base_exp ?? task.exp_gain ?? 0);
  if (task.dimension && task.dimension !== "san" && legacyExp) {
    effects.push({ dimension: task.dimension, delta: legacyExp });
  }
  const legacySan = Number(task.san_delta || 0);
  if (legacySan) effects.push({ dimension: "san", delta: legacySan });
  return effects;
}

export function sanHint(san) {
  if (san < 15) return t("san.hint.critical");
  if (san < 30) return t("san.hint.low");
  if (san < 55) return t("san.hint.mid");
  return t("san.hint.full");
}

export function dailySanText() {
  const state = getState();
  const daily = state.daily_san || {};
  const history = Object.entries(daily.history || {}).sort(([a], [b]) => a.localeCompare(b));
  const last = history.length ? history[history.length - 1] : null;
  const current = Math.round(Number(daily.current ?? state.derived.san) * 10) / 10;
  const change = formatDelta(daily.change ?? 0);
  return last
    ? t("san.daily.summaryWithHistory", { current, change, date: last[0], final: last[1] })
    : t("san.daily.summary", { current, change });
}
