// 模态框：新建/编辑里程碑、新建/编辑 Task（多属性效果选择器）、结项祭坛与表单提交。

import { $$, escapeHtml } from "./utils.js";
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

export function openEpicModal(epicId = null) {
  const epic = epicId ? getState().epics[epicId] : null;
  if (epicId && !epic) return;
  const optionsFor = (selected) =>
    poolDimensionOrder
      .map((key) => `<option value="${key}" ${selected === key ? "selected" : ""}>${dimensionName(key)}</option>`)
      .join("");
  openModal(
    epic ? "编辑里程碑" : "建立里程碑",
    `<form class="form-grid" data-form="epic" data-id="${epic?.id || ""}">
      <div class="form-field"><label>里程碑名称</label><input name="title" required value="${escapeHtml(epic?.title || "")}" placeholder="例如：上线企业级知识检索系统" /></div>
      <div class="form-field"><label>描述</label><textarea name="description" placeholder="它完成后的样子是什么？">${escapeHtml(epic?.description || "")}</textarea></div>
      <div class="form-field"><label>主维度</label><select name="main_dimension">${optionsFor(epic?.main_dimension || "professional")}</select></div>
      <div class="form-field"><label>对应称号加成属性</label><select name="title_bonus_dimension">${optionsFor(epic?.title_bonus_dimension || "professional")}</select></div>
      <div class="form-field"><label>称号加成百分比</label><input name="title_bonus_percent" type="number" step="1" value="${Number(epic?.title_bonus_percent || 10)}" /></div>
      <div class="form-field"><label>称号图标</label><input name="title_emoji" value="${escapeHtml(epic?.title_emoji || "🏅")}" maxlength="4" /></div>
      <div class="form-actions"><button class="ghost-button" type="button" data-close-modal>取消</button><button class="primary-button" type="submit">${epic ? "保存" : "创建"}</button></div>
    </form>`
  );
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
        `<option value="${epic.id}" ${task?.epic_id === epic.id ? "selected" : ""}>${escapeHtml(epic.title)}${epic.status === "completed" ? "（已结项）" : ""}</option>`
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
    task ? "编辑 Task" : "新建 Task",
    `<form class="form-grid" data-form="task" data-id="${task?.id || ""}">
      <p class="muted">勾选一个或多个属性，填写增益或减益数值。右侧会实时显示该属性的当前数据和变更后的预览。</p>
      <div class="form-field"><label>Task 名称</label><input name="title" required value="${escapeHtml(task?.title || "")}" placeholder="例如：早起 / 阅读半小时 / 熬夜" /></div>
      <div class="form-field"><label>归属</label><select name="epic_id">${`<option value="" ${task?.epic_id ? "" : "selected"}>独立 Task（不挂里程碑）</option>` + epicOptions}</select></div>
      <div class="form-field">
        <label>属性效果</label>
        <div class="effect-picker">${effectOptions}</div>
      </div>
      <div class="form-field"><label><input name="repeatable" type="checkbox" ${task?.repeatable ? "checked" : ""} /> 可重复记录（适合早起、阅读、熬夜等日常 Task）</label></div>
      <div class="form-actions"><button class="ghost-button" type="button" data-close-modal>取消</button><button class="primary-button" type="submit">${task ? "保存" : "创建"}</button></div>
    </form>`
  );
  bindEffectPicker();
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
    "结项祭坛",
    `<p class="muted">${escapeHtml(epic.title)} 已经走完一段路。请写下 1-2 句结项铭文，作为受封证据。</p>
    <form class="form-grid" data-form="complete-epic" data-id="${epicId}">
      <div class="form-field"><label>结项铭文</label><textarea name="engraving" required placeholder="例如：把复杂问题拆成可执行的下一步，系统就长出来了。"></textarea></div>
      <div class="form-actions"><button class="ghost-button" type="button" data-close-modal>取消</button><button class="danger-button" type="submit">敲下回车 · 结项</button></div>
    </form>`
  );
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
      toast(epicId ? "里程碑已更新。" : "里程碑已建立。");
    } else if (form.dataset.form === "task") {
      const effects = $$("[data-effect-toggle]:checked")
        .map((toggle) => ({
          dimension: toggle.value,
          delta: Number(document.querySelector(`[data-effect-delta="${toggle.value}"]`).value || 0),
        }))
        .filter((effect) => effect.delta !== 0);
      if (!effects.length) {
        toast("至少选择一个属性并填写非零数值。", true);
        return;
      }
      data.repeatable = formData.get("repeatable") === "on";
      data.effects = effects;
      const taskId = form.dataset.id;
      await api(taskId ? `/api/tasks/${taskId}/update` : "/api/tasks", {
        method: "POST",
        body: JSON.stringify(data),
      });
      toast(taskId ? "Task 已更新。" : "Task 已建立。");
    } else if (form.dataset.form === "complete-epic") {
      await api(`/api/epics/${form.dataset.id}/complete`, {
        method: "POST",
        body: JSON.stringify({ engraving: data.engraving }),
      });
      toast("里程碑已结项，对应称号已解锁。");
    }
    closeModal();
    await loadState();
  } catch (error) {
    toast(error.message, true);
  }
}
