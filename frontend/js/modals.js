// 模态框：新建/编辑里程碑、新建/编辑 Task（多属性效果选择器）、结项祭坛与表单提交。

import { t, onLocaleChanged } from "./i18n/index.js";
import { $, $$, escapeHtml } from "./utils.js";
import { getState, poolDimensionOrder, taskEffectDimensionOrder } from "./state.js";
import {
  dimensionName,
  attributeValueLabel,
  projectedAttributeLabel,
  taskEffects,
} from "./dimensions.js";
import { openModal, closeModal, toast } from "./ui.js";
import { api } from "./api.js";
import { loadState } from "./controller.js";

// 当前打开的业务模态（语言切换时按原参数重建并恢复未提交输入）。
let currentModal = null;

export function openEpicModal(epicId = null) {
  const epic = epicId ? getState().epics[epicId] : null;
  if (epicId && !epic) return;
  const optionsFor = (selected) =>
    poolDimensionOrder
      .map((key) => `<option value="${key}" ${selected === key ? "selected" : ""}>${dimensionName(key)}</option>`)
      .join("");
  openModal(
    epic ? t("modal.epic.titleEdit") : t("modal.epic.titleCreate"),
    `<form class="form-grid" data-form="epic" data-id="${epic?.id || ""}">
      <div class="form-field"><label>${t("modal.epic.titleLabel")}</label><input name="title" required value="${escapeHtml(epic?.title || "")}" placeholder="${t("modal.epic.titlePh")}" /></div>
      <div class="form-field"><label>${t("modal.epic.descriptionLabel")}</label><textarea name="description" placeholder="${t("modal.epic.descriptionPh")}">${escapeHtml(epic?.description || "")}</textarea></div>
      <div class="form-field"><label>${t("modal.epic.mainDimension")}</label><select name="main_dimension">${optionsFor(epic?.main_dimension || "professional")}</select></div>
      <div class="form-field"><label>${t("modal.epic.bonusDimension")}</label><select name="title_bonus_dimension">${optionsFor(epic?.title_bonus_dimension || "professional")}</select></div>
      <div class="form-field"><label>${t("modal.epic.bonusPercent")}</label><input name="title_bonus_percent" type="number" step="1" value="${Number(epic?.title_bonus_percent || 10)}" /></div>
      <div class="form-field"><label>${t("modal.epic.titleIcon")}</label><input name="title_emoji" value="${escapeHtml(epic?.title_emoji || "🏅")}" maxlength="4" /></div>
      <div class="form-actions"><button class="ghost-button" type="button" data-close-modal>${t("common.cancel")}</button><button class="primary-button" type="submit">${epic ? t("common.save") : t("common.create")}</button></div>
    </form>`
  );
  currentModal = { kind: "epic", id: epicId };
}

export function openTaskModal(taskId = null) {
  const task = taskId ? getState().tasks[taskId] : null;
  if (taskId && !task) return;
  const existingEffects = new Map(
    (task ? taskEffects(task) : []).map((effect) => [effect.dimension, Number(effect.delta)])
  );
  const allEpics = Object.values(getState().epics);
  const epicOptions = allEpics
    .map(
      (epic) =>
        `<option value="${epic.id}" ${task?.epic_id === epic.id ? "selected" : ""}>${epic.status === "completed" ? t("modal.task.epicCompleted", { title: escapeHtml(epic.title) }) : escapeHtml(epic.title)}</option>`
    )
    .join("");
  const effectOptions = taskEffectDimensionOrder
    .map((dimension) => {
      const checked = existingEffects.has(dimension);
      const delta = checked ? existingEffects.get(dimension) : 0;
      return `
      <div class="effect-option">
        <label class="effect-toggle">
          <input type="checkbox" data-effect-toggle value="${dimension}" ${checked ? "checked" : ""} />
          <span>${dimensionName(dimension)}</span>
        </label>
        <span class="effect-current">${attributeValueLabel(dimension)}</span>
        <input class="effect-delta-input" data-effect-delta="${dimension}" type="number" step="1" value="${delta}" ${checked ? "" : "disabled"} />
        <span class="effect-preview" data-effect-preview="${dimension}">${projectedAttributeLabel(dimension, checked ? delta : 0)}</span>
      </div>
    `;
    })
    .join("");
  openModal(
    task ? t("modal.task.titleEdit") : t("modal.task.titleCreate"),
    `<form class="form-grid" data-form="task" data-id="${task?.id || ""}">
      <p class="muted">${t("modal.task.intro")}</p>
      <div class="form-field"><label>${t("modal.task.titleLabel")}</label><input name="title" required value="${escapeHtml(task?.title || "")}" placeholder="${t("modal.task.titlePh")}" /></div>
      <div class="form-field"><label>${t("modal.task.epicLabel")}</label><select name="epic_id">${`<option value="" ${task?.epic_id ? "" : "selected"}>${t("modal.task.independent")}</option>` + epicOptions}</select></div>
      <div class="form-field">
        <label>${t("modal.task.effects")}</label>
        <div class="effect-picker">${effectOptions}</div>
      </div>
      <div class="form-field"><label><input name="repeatable" type="checkbox" ${task?.repeatable ? "checked" : ""} /> ${t("modal.task.repeatable")}</label></div>
      <div class="form-actions"><button class="ghost-button" type="button" data-close-modal>${t("common.cancel")}</button><button class="primary-button" type="submit">${task ? t("common.save") : t("common.create")}</button></div>
    </form>`
  );
  bindEffectPicker();
  currentModal = { kind: "task", id: taskId };
}

function bindEffectPicker() {
  $$("[data-effect-toggle]").forEach((toggle) => {
    toggle.addEventListener("change", () => {
      const dimension = toggle.value;
      const input = document.querySelector(`[data-effect-delta="${dimension}"]`);
      input.disabled = !toggle.checked;
      if (toggle.checked && Number(input.value) === 0) {
        input.value = "5";
      }
      updateEffectPreview(dimension);
    });
  });
  $$("[data-effect-delta]").forEach((input) => {
    input.addEventListener("input", () => updateEffectPreview(input.dataset.effectDelta));
  });
  taskEffectDimensionOrder.forEach(updateEffectPreview);
}

function updateEffectPreview(dimension) {
  const toggle = document.querySelector(`[data-effect-toggle][value="${dimension}"]`);
  const input = document.querySelector(`[data-effect-delta="${dimension}"]`);
  const preview = document.querySelector(`[data-effect-preview="${dimension}"]`);
  if (!toggle || !input || !preview) return;
  const delta = toggle.checked ? Number(input.value || 0) : 0;
  preview.textContent = projectedAttributeLabel(dimension, delta);
  preview.className = `effect-preview ${delta > 0 ? "positive" : delta < 0 ? "negative" : ""}`;
}

export function openCompleteEpicModal(epicId) {
  const epic = getState().epics[epicId];
  if (!epic) return;
  openModal(
    t("modal.complete.title"),
    `<p class="muted">${t("modal.complete.intro", { title: escapeHtml(epic.title) })}</p>
    <form class="form-grid" data-form="complete-epic" data-id="${epicId}">
      <div class="form-field"><label>${t("modal.complete.engravingLabel")}</label><textarea name="engraving" required placeholder="${t("modal.complete.engravingPh")}"></textarea></div>
      <div class="form-actions"><button class="ghost-button" type="button" data-close-modal>${t("common.cancel")}</button><button class="danger-button" type="submit">${t("modal.complete.submit")}</button></div>
    </form>`
  );
  currentModal = { kind: "complete", id: epicId };
}

// 语言切换时若业务模态打开：按原参数重建（新语言文案）并恢复未提交的表单输入。
onLocaleChanged(() => {
  if (!currentModal) return;
  if ($("#modal").hidden) {
    currentModal = null;
    return;
  }
  const snapshot = captureModalSnapshot();
  if (currentModal.kind === "epic") openEpicModal(currentModal.id);
  else if (currentModal.kind === "task") openTaskModal(currentModal.id);
  else if (currentModal.kind === "complete") openCompleteEpicModal(currentModal.id);
  restoreModalSnapshot(snapshot);
});

// 捕获全部命名控件 + 属性效果选择器状态（disabled 输入不含于 FormData，须单独取）。
function captureModalSnapshot() {
  const fields = {};
  $$("#modalBody [name]").forEach((element) => {
    fields[element.name] = element.type === "checkbox" ? element.checked : element.value;
  });
  const effects = {};
  $$("[data-effect-toggle]").forEach((toggle) => {
    const input = document.querySelector(`[data-effect-delta="${toggle.value}"]`);
    effects[toggle.value] = { checked: toggle.checked, delta: input ? input.value : "" };
  });
  return { fields, effects };
}

function restoreModalSnapshot(snapshot) {
  Object.entries(snapshot.fields).forEach(([name, value]) => {
    const element = document.querySelector(`#modalBody [name="${name}"]`);
    if (!element) return;
    if (element.type === "checkbox") element.checked = value;
    else element.value = value;
  });
  Object.entries(snapshot.effects).forEach(([dimension, state]) => {
    const toggle = document.querySelector(`[data-effect-toggle][value="${dimension}"]`);
    const input = document.querySelector(`[data-effect-delta="${dimension}"]`);
    if (!toggle || !input) return;
    toggle.checked = state.checked;
    input.disabled = !state.checked;
    input.value = state.delta;
    updateEffectPreview(dimension);
  });
}

export async function handleModalSubmit(event) {
  event.preventDefault();
  const form = event.target.closest("form");
  const formData = new FormData(form);
  const data = Object.fromEntries(formData.entries());
  try {
    if (form.dataset.form === "epic") {
      const epicId = form.dataset.id;
      await api(epicId ? `/api/epics/${epicId}/update` : "/api/epics", {
        method: "POST",
        body: JSON.stringify(data),
      });
      toast(epicId ? t("toast.epicUpdated") : t("toast.epicCreated"));
    } else if (form.dataset.form === "task") {
      const effects = $$("[data-effect-toggle]:checked")
        .map((toggle) => ({
          dimension: toggle.value,
          delta: Number(document.querySelector(`[data-effect-delta="${toggle.value}"]`).value || 0),
        }))
        .filter((effect) => effect.delta !== 0);
      if (!effects.length) {
        toast(t("toast.taskNeedEffect"), true);
        return;
      }
      data.repeatable = formData.get("repeatable") === "on";
      data.effects = effects;
      const taskId = form.dataset.id;
      await api(taskId ? `/api/tasks/${taskId}/update` : "/api/tasks", {
        method: "POST",
        body: JSON.stringify(data),
      });
      toast(taskId ? t("toast.taskUpdated") : t("toast.taskCreated"));
    } else if (form.dataset.form === "complete-epic") {
      await api(`/api/epics/${form.dataset.id}/complete`, {
        method: "POST",
        body: JSON.stringify({ engraving: data.engraving }),
      });
      toast(t("toast.epicCompleted"));
    }
    closeModal();
    await loadState();
  } catch (error) {
    toast(error.message, true);
  }
  if ($("#modal").hidden) currentModal = null;
}
