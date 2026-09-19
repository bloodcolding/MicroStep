// 六维雷达图：Canvas 2D 手绘，对数归一化。

import { getState, radarDimensionOrder } from "../state.js";
import { dimensionName } from "../dimensions.js";
import { clamp } from "../utils.js";

export function renderRadar() {
  const state = getState();
  const canvas = document.getElementById("radarCanvas");
  if (!canvas) return;
  const ctx = canvas.getContext("2d");
  const dpr = window.devicePixelRatio || 1;
  const cssWidth = canvas.clientWidth || 520;
  const cssHeight = canvas.clientHeight || 460;
  canvas.width = cssWidth * dpr;
  canvas.height = cssHeight * dpr;
  ctx.scale(dpr, dpr);

  const width = cssWidth;
  const height = cssHeight;
  const cx = width / 2;
  const cy = height / 2;
  const radius = Math.min(width, height) * 0.34;
  const count = radarDimensionOrder.length;

  ctx.clearRect(0, 0, width, height);
  ctx.font = "12px Inter, PingFang SC, Microsoft YaHei";
  ctx.textAlign = "center";
  ctx.textBaseline = "middle";

  for (let ring = 1; ring <= 4; ring += 1) {
    ctx.beginPath();
    for (let i = 0; i < count; i += 1) {
      const angle = (Math.PI * 2 * i) / count - Math.PI / 2;
      const r = radius * (ring / 4);
      const x = cx + Math.cos(angle) * r;
      const y = cy + Math.sin(angle) * r;
      if (i === 0) ctx.moveTo(x, y);
      else ctx.lineTo(x, y);
    }
    ctx.closePath();
    ctx.strokeStyle = "rgba(255,255,255,0.10)";
    ctx.lineWidth = 1;
    ctx.stroke();
  }

  for (let i = 0; i < count; i += 1) {
    const angle = (Math.PI * 2 * i) / count - Math.PI / 2;
    ctx.beginPath();
    ctx.moveTo(cx, cy);
    ctx.lineTo(cx + Math.cos(angle) * radius, cy + Math.sin(angle) * radius);
    ctx.strokeStyle = "rgba(255,255,255,0.10)";
    ctx.stroke();

    const label = dimensionName(radarDimensionOrder[i]);
    const lx = cx + Math.cos(angle) * (radius + 30);
    const ly = cy + Math.sin(angle) * (radius + 30);
    ctx.fillStyle = "#aab6cc";
    ctx.fillText(label, lx, ly);
  }

  const values = radarDimensionOrder.map((key) => {
    const value = Number(state.dimensions[key] || 0);
    return Math.log10(value + 1) / Math.log10(10001);
  });
  ctx.beginPath();
  radarDimensionOrder.forEach((key, i) => {
    const angle = (Math.PI * 2 * i) / count - Math.PI / 2;
    const r = radius * clamp(values[i], 0.03, 1);
    const x = cx + Math.cos(angle) * r;
    const y = cy + Math.sin(angle) * r;
    if (i === 0) ctx.moveTo(x, y);
    else ctx.lineTo(x, y);
  });
  ctx.closePath();
  const gradient = ctx.createLinearGradient(0, 0, width, height);
  gradient.addColorStop(0, "rgba(110,231,183,0.30)");
  gradient.addColorStop(1, "rgba(125,211,252,0.24)");
  ctx.fillStyle = gradient;
  ctx.fill();
  ctx.strokeStyle = "#6ee7b7";
  ctx.lineWidth = 2;
  ctx.stroke();

  radarDimensionOrder.forEach((key, i) => {
    const angle = (Math.PI * 2 * i) / count - Math.PI / 2;
    const r = radius * clamp(values[i], 0.03, 1);
    ctx.beginPath();
    ctx.arc(cx + Math.cos(angle) * r, cy + Math.sin(angle) * r, 3, 0, Math.PI * 2);
    ctx.fillStyle = "#e8fff3";
    ctx.fill();
  });
}
