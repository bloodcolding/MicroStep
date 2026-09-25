// 用户动作：完成 Task、删除 Task/事件、装备称号、觉醒。成功后统一 loadState 刷新。

import { t } from "./i18n/index.js";
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
    toast(t("action.sanInsufficientToast", { san: state.derived.san, need: Math.abs(sanDelta) }), true);
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
      .join(t("common.listSeparator"));
    toast(t("action.taskLogged", { title: task.title, effects: effectText }));
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
    t("action.deleteTaskConfirm", { title: task.title })
  );
  if (!confirmed) return;
  try {
    await api(`/api/tasks/${taskId}/delete`, {
      method: "POST",
      body: JSON.stringify({}),
    });
    toast(t("action.taskDeleted"));
    await loadState();
  } catch (error) {
    toast(error.message, true);
  }
}

export async function deleteEvent(eventId) {
  if (!eventId) return;
  const confirmed = window.confirm(
    t("action.deleteEventConfirm")
  );
  if (!confirmed) return;
  try {
    await api(`/api/events/${encodeURIComponent(eventId)}/delete`, {
      method: "POST",
      body: JSON.stringify({}),
    });
    toast(t("action.eventDeleted"));
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
    toast(t("action.titleEquipped"));
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
    toast(t("action.titleUnequipped"));
    await loadState();
  } catch (error) {
    toast(error.message, true);
  }
}

export async function awaken() {
  try {
    await api("/api/awaken", { method: "POST", body: JSON.stringify({}) });
    toast(t("action.awakened"));
    await loadState();
  } catch (error) {
    toast(error.message, true);
  }
}
