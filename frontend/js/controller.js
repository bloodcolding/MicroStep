// 数据编排：加载 meta/state、驱动渲染入口。actions/modals 修改数据后调用 loadState 刷新。

import { $ } from "./utils.js";
import { fetchMeta, fetchState } from "./api.js";
import { getState, getMeta, setState, setMeta } from "./state.js";
import { renderSimple } from "./render/simple.js";
import { renderDashboard } from "./render/dashboard.js";

export async function loadMeta() {
  setMeta(await fetchMeta());
}

export async function loadState() {
  setState(await fetchState());
  renderAll();
}

export function renderAll() {
  if (!getState() || !getMeta()) return;
  const awakened = Boolean(getState().profile.awakened);
  $("#simpleView").hidden = awakened;
  $("#dashboardView").hidden = !awakened;
  $("#awakenTopBtn").hidden = awakened;
  renderSimple();
  renderDashboard();
}
