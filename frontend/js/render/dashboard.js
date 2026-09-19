// 觉醒后仪表盘：SAN 槽 + 属性卡 + 雷达 + 里程碑/Task/称号/事件流调度。

import { $, clamp } from "../utils.js";
import { getState } from "../state.js";
import { sanHint, dailySanText } from "../dimensions.js";
import { renderDimensionCards } from "./cards.js";
import { renderRadar } from "./radar.js";
import { renderEpics } from "./epics.js";
import { renderTasks } from "./tasks.js";
import { renderTitles } from "./titles.js";
import { renderEvents } from "./events.js";

export function renderDashboard() {
  const state = getState();
  const san = Number(state.derived.san);
  $("#sanValue").textContent = san;
  $("#sanBar").style.width = `${clamp(san, 0, 100)}%`;
  $("#sanHint").textContent = `${sanHint(san)} ${dailySanText()}`;
  renderDimensionCards();
  renderRadar();
  renderEpics();
  renderTasks();
  renderTitles();
  renderEvents("dashboardEvents", 18);
}
