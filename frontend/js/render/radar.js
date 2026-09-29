// 六维雷达图：Canvas 2D 手绘，对数归一化；尺寸随容器与 devicePixelRatio 自适应。

import { getState, radarDimensionOrder } from "../state.js";
import { dimensionName } from "../dimensions.js";
import { clamp } from "../utils.js";

// 上次绘制尺寸：ResizeObserver 回调据此跳过尺寸未变化的重绘。
const lastSize = { width: 0, height: 0, dpr: 1 };

// 容器尺寸 / 屏幕方向 / 缩放比例变化时重绘；不支持 ResizeObserver 的旧 WebView 回退 window resize。
export function observeRadarResize() {
  const canvas = document.getElementById("radarCanvas");
  if (!canvas) return;
  const redrawIfResized = () => {
    const width = Math.round(canvas.clientWidth);
    const height = Math.round(canvas.clientHeight);
    if (!width || !height) return;
    if (width === lastSize.width && height === lastSize.height && (window.devicePixelRatio || 1) === lastSize.dpr) return;
    // rAF 延迟到布局稳定后重绘，避免 ResizeObserver 同帧反馈。
    window.requestAnimationFrame(() => {
      if (Math.round(canvas.clientWidth) !== width || Math.round(canvas.clientHeight) !== height) return;
      renderRadar();
    });
  };
  if (typeof ResizeObserver === "function") {
    new ResizeObserver(redrawIfResized).observe(canvas);
  } else {
    window.addEventListener("resize", redrawIfResized);
  }
}
export function renderRadar() {
  const state = getState();
  const canvas = document.getElementById("radarCanvas");
  if (!canvas) return;
  const ctx = canvas.getContext("2d");
  const dpr = window.devicePixelRatio || 1;
  const cssWidth = canvas.clientWidth || 520;
  const cssHeight = canvas.clientHeight || 460;
  canvas.width = Math.round(cssWidth * dpr);
  canvas.height = Math.round(cssHeight * dpr);
  lastSize.width = Math.round(cssWidth);
  lastSize.height = Math.round(cssHeight);
  lastSize.dpr = dpr;
  ctx.scale(dpr, dpr);

  const width = cssWidth;
  const height = cssHeight;
  const cx = width / 2;
  const cy = height / 2;
  const count = radarDimensionOrder.length;

  ctx.clearRect(0, 0, width, height);
  const compact = Math.min(width, height) < 360;
  const fontSize = compact ? 11 : 12;
  const labels = radarDimensionOrder.map((key) => dimensionName(key));
  ctx.font = `${fontSize}px Inter, PingFang SC, Microsoft YaHei`;
  ctx.textAlign = "center";
  ctx.textBaseline = "middle";
  const maxHalfLabel = Math.max(...labels.map((label) => ctx.measureText(label).width)) / 2;
  const labelGap = Math.min(30, Math.max(16, Math.min(width, height) * 0.08));
  // 小屏收缩字号 / 半径 / 标签外边距，并按标签实测宽度反解半径，保证六维标签不被容器裁剪。
  const radius = Math.max(
    24,
    Math.min(
      Math.min(width, height) * (compact ? 0.3 : 0.34),
      width / 2 - maxHalfLabel - labelGap - 8,
      height / 2 - labelGap - fontSize - 8
    )
  );

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

    const label = labels[i];
    const lx = cx + Math.cos(angle) * (radius + labelGap);
    const ly = cy + Math.sin(angle) * (radius + labelGap);
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
