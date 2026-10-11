// 六维雷达图：Canvas 2D 手绘，对数归一化。

import { getState, radarDimensionOrder } from "../state.js";
import { dimensionName } from "../dimensions.js";
import { clamp } from "../utils.js";

// 上次绘制的 CSS 尺寸：供 ResizeObserver 回调跳过尺寸未变的重复重绘。
let lastPaintedSize = "";

export function renderRadar() {
  const state = getState();
  const canvas = document.getElementById("radarCanvas");
  if (!canvas) return;
  const cssWidth = canvas.clientWidth;
  const cssHeight = canvas.clientHeight;
  // 视图隐藏（无 CSS 盒）时不绘制，显示后由 ResizeObserver / 全量渲染触发重绘。
  if (!cssWidth || !cssHeight) return;
  const ctx = canvas.getContext("2d");
  const dpr = window.devicePixelRatio || 1;
  canvas.width = Math.round(cssWidth * dpr);
  canvas.height = Math.round(cssHeight * dpr);
  ctx.setTransform(dpr, 0, 0, dpr, 0, 0);

  const width = cssWidth;
  const height = cssHeight;
  const cx = width / 2;
  const cy = height / 2;
  const radius = Math.min(width, height) * 0.34;
  const count = radarDimensionOrder.length;
  // 标签外边距与字号随容器收缩：小屏六维标签不被卡片 / Canvas 边界裁剪（替代固定 30px）。
  const labelMargin = Math.max(14, Math.min(30, Math.min(width, height) * 0.07));
  const labelFont = Math.max(10, Math.min(13, radius * 0.14));

  ctx.clearRect(0, 0, width, height);
  ctx.font = `${labelFont}px Inter, PingFang SC, Microsoft YaHei`;
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
    const lx = cx + Math.cos(angle) * (radius + labelMargin);
    const ly = cy + Math.sin(angle) * (radius + labelMargin);
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

  lastPaintedSize = `${cssWidth}x${cssHeight}`;
}

// 容器尺寸 / 屏幕方向变化时自适应重绘：优先 ResizeObserver，缺失时回退 window resize。
function initRadarResize() {
  const canvas = document.getElementById("radarCanvas");
  if (!canvas) return;
  if (window.ResizeObserver) {
    new ResizeObserver(() => {
      if (`${canvas.clientWidth}x${canvas.clientHeight}` !== lastPaintedSize) {
        renderRadar();
      }
    }).observe(canvas);
  } else {
    window.addEventListener("resize", renderRadar);
  }
}

initRadarResize();
