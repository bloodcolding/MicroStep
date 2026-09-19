// 初见期简单视图：只显示 SAN 血条与最近事件。

import { $, clamp } from "../utils.js";
import { getState } from "../state.js";
import { dailySanText } from "../dimensions.js";
import { renderEvents } from "./events.js";

export function renderSimple() {
  const state = getState();
  const san = Number(state.derived.san);
  $("#simpleSanValue").textContent = san;
  $("#simpleSanBar").style.width = `${clamp(san, 0, 100)}%`;
  $("#simpleSanHint").textContent = dailySanText();
  renderEvents("simpleEvents", 6);
}
