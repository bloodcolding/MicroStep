// 应用状态单例：state/meta 来自后端重放，搜索词与显示开关是本地 UI 状态。

let state = null;
let meta = null;
let taskSearchQuery = "";
let epicSearchQuery = "";

let showCompletedTasks = false;
try {
  showCompletedTasks = window.localStorage.getItem("microstep.showCompletedTasks") === "true";
} catch {
  showCompletedTasks = false;
}

// 六维池属性的展示顺序（属性卡 / 雷达图共用）。
export const poolDimensionOrder = [
  "physical",
  "professional",
  "knowledge",
  "expression",
  "kindness",
  "charm",
];

export const radarDimensionOrder = ["physical", "professional", "knowledge", "expression", "kindness", "charm"];

// Task 属性效果编辑顺序：SAN 在最前。
export const taskEffectDimensionOrder = ["san", "physical", "professional", "knowledge", "expression", "kindness", "charm"];

export function getState() {
  return state;
}

export function setState(next) {
  state = next;
}

export function getMeta() {
  return meta;
}

export function setMeta(next) {
  meta = next;
}

export function getTaskSearchQuery() {
  return taskSearchQuery;
}

export function setTaskSearchQuery(value) {
  taskSearchQuery = value;
}

export function getEpicSearchQuery() {
  return epicSearchQuery;
}

export function setEpicSearchQuery(value) {
  epicSearchQuery = value;
}

export function isShowCompletedTasks() {
  return showCompletedTasks;
}

export function setShowCompletedTasks(value) {
  showCompletedTasks = Boolean(value);
  try {
    window.localStorage.setItem("microstep.showCompletedTasks", String(showCompletedTasks));
  } catch {
    // 本地存储不可用时只保留当前会话状态。
  }
}
