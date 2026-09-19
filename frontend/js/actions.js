// 用户动作：完成 Task、删除 Task/事件、装备称号、觉醒。成功后统一 loadState 刷新。

import { api } from "./api.js";
import { getState } from "./state.js";
import { toast } from "./ui.js";
import { loadState } from "./controller.js";
import { dimensionName, taskEffects } from "./dimensions.js";
import { formatDelta } from "./utils.js";

export async function completeTask(taskId) {
  const state = getState();
  const task = state.tasks[taskId];
  if (!task) return;
  const effects = taskEffects(task);
  const sanDelta = effects
    .filter((effect) => effect.dimension === "san")
    .reduce((total, effect) => total + Number(effect.delta || 0), 0);
  if (sanDelta < 0 && Number(state.derived.san) + sanDelta < 0) {
    toast(`SAN 不足：当前 ${state.derived.san}，该 Task 需要 ${Math.abs(sanDelta)}。`, true);
    return;
  }
  try {
    const result = await api(`/api/tasks/${taskId}/log`, {
      method: "POST",
      body: JSON.stringify({ note: "" }),
    });
    const appliedEffects = Array.isArray(result.event?.effects) ? result.event.effects : effects;
    const effectText = appliedEffects
      .map((effect) => `${dimensionName(effect.dimension)} ${formatDelta(effect.delta)}`)
      .join("，");
    toast(`已记录：${task.title}，${effectText}`);
    await loadState();
  } catch (error) {
    toast(error.message, true);
  }
}

export async function deleteTask(taskId) {
  if (!taskId) return;
  const task = getState().tasks[taskId];
  if (!task) return;
  const confirmed = window.confirm(
    `删除 Task「${task.title}」？历史事件会保留，只追加 TASK_DELETED 标记，之后不再出现在 Task 列表中。`
  );
  if (!confirmed) return;
  try {
    await api(`/api/tasks/${taskId}/delete`, {
      method: "POST",
      body: JSON.stringify({}),
    });
    toast("Task 已删除，历史事件仍保留。");
    await loadState();
  } catch (error) {
    toast(error.message, true);
  }
}

export async function deleteEvent(eventId) {
  if (!eventId) return;
  const confirmed = window.confirm(
    "删除这条事件？原始事件会保留，不会影响 Task 或属性状态。"
  );
  if (!confirmed) return;
  try {
    await api(`/api/events/${encodeURIComponent(eventId)}/delete`, {
      method: "POST",
      body: JSON.stringify({}),
    });
    toast("事件已删除，原始数据仍然保留。");
    await loadState();
  } catch (error) {
    toast(error.message, true);
  }
}

export async function equipTitle(titleId) {
  try {
    await api("/api/titles/equip", {
      method: "POST",
      body: JSON.stringify({ title_id: titleId }),
    });
    toast("称号已装备。");
    await loadState();
  } catch (error) {
    toast(error.message, true);
  }
}

export async function unequipTitle(titleId) {
  try {
    await api("/api/titles/unequip", {
      method: "POST",
      body: JSON.stringify({ title_id: titleId }),
    });
    toast("称号已卸下。");
    await loadState();
  } catch (error) {
    toast(error.message, true);
  }
}

export async function awaken() {
  try {
    await api("/api/awaken", { method: "POST", body: JSON.stringify({}) });
    toast("八维雷达已觉醒。");
    await loadState();
  } catch (error) {
    toast(error.message, true);
  }
}
