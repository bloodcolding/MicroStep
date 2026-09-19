// 维度取值、预览与 Task 效果读取：连接 state/meta 与各渲染模块。

import { getState, getMeta } from "./state.js";
import { clamp, formatDelta } from "./utils.js";

export function dimensionName(key) {
  return getMeta()?.dimensions?.[key]?.name || key;
}

export function attributeCurrentValue(dimension) {
  const state = getState();
  if (dimension === "san") return Number(state.dimensions.san || 0);
  return Number(state.dimensions[dimension] || 0);
}

export function attributeValueLabel(dimension) {
  const value = attributeCurrentValue(dimension);
  if (dimension === "san") return `${value} / 100`;
  return value.toLocaleString("zh-CN", { maximumFractionDigits: 1 });
}

export function projectedAttributeLabel(dimension, delta) {
  const value = attributeCurrentValue(dimension);
  const projected = dimension === "san"
    ? clamp(value + delta, 0, 100)
    : Math.max(0, value + delta);
  const unit = dimension === "san" ? " / 100" : "";
  return `→ ${projected.toLocaleString("zh-CN", { maximumFractionDigits: 1 })}${unit}`;
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
  if (san < 15) return "⚠️ SAN 告急：当前无法结算任何扣减 SAN 的 Task，请优先休息恢复。";
  if (san < 30) return "🪫 SAN 低水位：继续扣减可能触发结算拒绝，建议先安排恢复。";
  if (san < 55) return "⚖️ 精力中位，注意安排恢复。";
  return "✨ 精力充足，适合处理高负荷任务。";
}

export function dailySanText() {
  const state = getState();
  const daily = state.daily_san || {};
  const history = Object.entries(daily.history || {}).sort(([a], [b]) => a.localeCompare(b));
  const last = history.length ? history[history.length - 1] : null;
  const base = `今日起始 100 · 当前 ${Math.round(Number(daily.current ?? state.derived.san) * 10) / 10} · 今日变化 ${formatDelta(daily.change ?? 0)}`;
  return last ? `${base} · ${last[0]} 最终 SAN ${last[1]}` : base;
}
