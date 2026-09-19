// 六维属性卡渲染：对数尺度条形图。

import { getState, getMeta, poolDimensionOrder } from "../state.js";
import { dimensionName } from "../dimensions.js";
import { clamp } from "../utils.js";

export function renderDimensionCards() {
  const state = getState();
  const meta = getMeta();
  const host = document.getElementById("dimensionCards");
  if (!host) return;
  host.innerHTML = "";
  poolDimensionOrder.forEach((key) => {
    const value = Number(state.dimensions[key] || 0);
    const card = document.createElement("div");
    card.className = "dimension-card";
    card.innerHTML = `
      <div class="top">
        <span class="name">${dimensionName(key)}</span>
        <span class="value">${value.toLocaleString("zh-CN", { maximumFractionDigits: 1 })}</span>
      </div>
      <div class="bar"><span class="bar-fill" style="width:${attributeBarWidth(value)}%;background:linear-gradient(90deg,#7dd3fc,#6ee7b7)"></span></div>
      <p class="gauge-hint">${meta.dimensions[key].full_name}</p>
    `;
    host.appendChild(card);
  });
}

function attributeBarWidth(value) {
  // 六维池满分参照 10000（与雷达图统一口径，对齐产品量纲天花板）。
  const ratio = Math.log10(value + 1) / Math.log10(10001);
  return Math.round(clamp(ratio * 100, 4, 100));
}
