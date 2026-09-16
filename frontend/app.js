const $ = (selector) => document.querySelector(selector);
const $$ = (selector) => Array.from(document.querySelectorAll(selector));

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

const poolDimensionOrder = [
  "physical",
  "professional",
  "knowledge",
  "expression",
  "kindness",
  "charm",
];

const radarDimensionOrder = ["physical", "professional", "knowledge", "expression", "kindness", "charm"];
const taskEffectDimensionOrder = ["san", "physical", "professional", "knowledge", "expression", "kindness", "charm"];

document.addEventListener("DOMContentLoaded", () => {
  $("#todayLabel").textContent = new Date().toLocaleDateString("zh-CN", {
    year: "numeric",
    month: "long",
    day: "numeric",
    weekday: "short",
  });
  bindEvents();
  loadMeta().then(loadState).catch((error) => toast(error.message, true));
});

function bindEvents() {
  $("#awakenTopBtn").addEventListener("click", awaken);
  $("#refreshBtn").addEventListener("click", () => loadState().catch((error) => toast(error.message, true)));
  $("#newEpicBtn").addEventListener("click", () => openEpicModal());
  $("#newTaskBtn").addEventListener("click", () => openTaskModal());
  $("#simpleNewTaskBtn").addEventListener("click", () => openTaskModal());
  $("#taskSearchInput").addEventListener("input", (event) => {
    taskSearchQuery = event.target.value.trim().toLowerCase();
    renderTasks();
  });
  $("#epicSearchInput").addEventListener("input", (event) => {
    epicSearchQuery = event.target.value.trim().toLowerCase();
    renderEpics();
  });
  const showCompletedToggle = $("#showCompletedTasks");
  showCompletedToggle.checked = showCompletedTasks;
  showCompletedToggle.addEventListener("change", (event) => {
    showCompletedTasks = event.target.checked;
    try {
      window.localStorage.setItem("microstep.showCompletedTasks", String(showCompletedTasks));
    } catch {
      // 本地存储不可用时只保留当前会话状态。
    }
    renderTasks();
  });
  $("#modal").addEventListener("click", (event) => {
    if (event.target.matches("[data-close-modal]")) closeModal();
  });
  $("#modalBody").addEventListener("submit", handleModalSubmit);
}

async function api(path, options = {}) {
  const response = await fetch(path, {
    headers: { "Content-Type": "application/json" },
    ...options,
  });
  let payload;
  try {
    payload = await response.json();
  } catch {
    throw new Error(`HTTP ${response.status}`);
  }
  if (!response.ok || payload.ok === false) {
    throw new Error(payload.error || `HTTP ${response.status}`);
  }
  return payload;
}

async function loadMeta() {
  meta = await api("/api/meta");
  return meta;
}

async function loadState() {
  state = await api("/api/state");
  renderAll();
  return state;
}

function renderAll() {
  if (!state || !meta) return;
  const awakened = Boolean(state.profile.awakened);
  $("#simpleView").hidden = awakened;
  $("#dashboardView").hidden = !awakened;
  $("#awakenTopBtn").hidden = awakened;
  renderSimple();
  renderDashboard();
}

function renderSimple() {
  const san = Number(state.derived.san);
  $("#simpleSanValue").textContent = san;
  $("#simpleSanBar").style.width = `${clamp(san, 0, 100)}%`;
  $("#simpleSanHint").textContent = dailySanText();
  renderEvents("simpleEvents", 6);
}

function renderDashboard() {
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

function sanHint(san) {
  if (san < 15) return "⚠️ 警戒状态：高负荷事件收益降至 25%，建议立即休息。";
  if (san < 30) return "🪫 疲劳状态：高负荷事件收益减半，恢复类事件有额外奖励。";
  if (san < 55) return "⚖️ 精力中位，注意安排恢复。";
  return "✨ 精力充足，适合处理高负荷任务。";
}

function dailySanText() {
  const daily = state.daily_san || {};
  const history = Object.entries(daily.history || {}).sort(([a], [b]) => a.localeCompare(b));
  const last = history.length ? history[history.length - 1] : null;
  const base = `今日起始 100 · 当前 ${Math.round(Number(daily.current ?? state.derived.san) * 10) / 10} · 今日变化 ${formatDelta(daily.change ?? 0)}`;
  return last ? `${base} · ${last[0]} 最终 SAN ${last[1]}` : base;
}

function dimensionName(key) {
  return meta?.dimensions?.[key]?.name || key;
}

function attributeCurrentValue(dimension) {
  if (dimension === "san") return Number(state.dimensions.san || 0);
  return Number(state.dimensions[dimension] || 0);
}

function attributeValueLabel(dimension) {
  const value = attributeCurrentValue(dimension);
  if (dimension === "san") return `${value} / 100`;
  return value.toLocaleString("zh-CN", { maximumFractionDigits: 1 });
}

function projectedAttributeLabel(dimension, delta) {
  const value = attributeCurrentValue(dimension);
  const projected = dimension === "san"
    ? clamp(value + delta, 0, 100)
    : Math.max(0, value + delta);
  const unit = dimension === "san" ? " / 100" : "";
  return `→ ${projected.toLocaleString("zh-CN", { maximumFractionDigits: 1 })}${unit}`;
}

function taskEffects(task) {
  if (Array.isArray(task.effects) && task.effects.length) return task.effects;
  const effects = [];
  const legacyExp = Number(task.exp_delta ?? task.base_exp ?? task.exp_gain ?? 0);
  if (task.dimension && task.dimension !== "san" && legacyExp) {
    effects.push({ dimension: task.dimension, delta: legacyExp });
  }
  const legacySan = Number(task.san_delta || 0);
  if (legacySan) effects.push({ dimension: "san", delta: legacySan });
  return effects;
}

function renderDimensionCards() {
  const host = $("#dimensionCards");
  host.innerHTML = "";
  poolDimensionOrder.forEach((key) => {
    const value = Number(state.dimensions[key] || 0);
    const card = document.createElement("div");
    card.className = "dimension-card";
    card.innerHTML = `
      <div class="top">
        <span class="name">${dimensionName(key)}</span>
        <span class="value">${value.toLocaleString("zh-CN", { maximumFractionDigits: 1 })}</span>
      </div>
      <div class="bar"><span class="bar-fill" style="width:${attributeBarWidth(value)}%;background:linear-gradient(90deg,#7dd3fc,#6ee7b7)"></span></div>
      <p class="gauge-hint">${meta.dimensions[key].full_name}</p>
    `;
    host.appendChild(card);
  });
}

function attributeBarWidth(value) {
  const ratio = Math.log10(value + 1) / Math.log10(1001);
  return Math.round(clamp(ratio * 100, 4, 100));
}

function renderRadar() {
  const canvas = $("#radarCanvas");
  if (!canvas) return;
  const ctx = canvas.getContext("2d");
  const dpr = window.devicePixelRatio || 1;
  const cssWidth = canvas.clientWidth || 520;
  const cssHeight = canvas.clientHeight || 460;
  canvas.width = cssWidth * dpr;
  canvas.height = cssHeight * dpr;
  ctx.scale(dpr, dpr);

  const width = cssWidth;
  const height = cssHeight;
  const cx = width / 2;
  const cy = height / 2;
  const radius = Math.min(width, height) * 0.34;
  const count = radarDimensionOrder.length;

  ctx.clearRect(0, 0, width, height);
  ctx.font = "12px Inter, PingFang SC, Microsoft YaHei";
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
    const lx = cx + Math.cos(angle) * (radius + 30);
    const ly = cy + Math.sin(angle) * (radius + 30);
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

function renderEpics() {
  const host = $("#epicList");
  host.innerHTML = "";
  const allEpics = Object.values(state.epics);
  const epics = allEpics
    .filter((epic) => epicMatchesQuery(epic, epicSearchQuery))
    .sort((a, b) => {
      if (a.status !== b.status) return a.status === "active" ? -1 : 1;
      return String(b.created_at).localeCompare(String(a.created_at));
    });
  const count = $("#epicCount");
  if (count) {
    count.textContent = epicSearchQuery
      ? `${epics.length} / ${allEpics.length} 个里程碑`
      : `${allEpics.length} 个里程碑`;
  }
  if (!epics.length) {
    host.innerHTML = epicSearchQuery
      ? '<p class="muted">没有匹配的里程碑。试试搜索名称、描述、属性或状态。</p>'
      : '<p class="muted">还没有里程碑。创建一个方向，让日常 Task 有处可归。</p>';
    return;
  }
  epics.forEach((epic) => {
    const card = document.createElement("article");
    card.className = `epic-card ${epic.status === "completed" ? "completed" : ""}`;
    const completed = epic.status === "completed";
    const taskCount = epic.task_ids?.length || 0;
    const bonusDimension = epic.title_bonus_dimension || epic.main_dimension;
    card.innerHTML = `
      <div class="epic-top">
        <div>
          <h4>${escapeHtml(epic.title)}</h4>
          <p>${escapeHtml(epic.description || "没有描述")}</p>
        </div>
        <div class="epic-actions">
          <button class="small-button" data-edit-epic="${epic.id}">编辑</button>
          ${
            completed
              ? '<span class="meta-pill">已结项</span>'
              : '<button class="small-button" data-complete-epic="' + epic.id + '">结项祭坛</button>'
          }
        </div>
      </div>
      <div class="epic-meta">
        <span class="meta-pill">${dimensionName(epic.main_dimension)}</span>
        <span class="meta-pill">称号加成：${dimensionName(bonusDimension)} ${formatPercent(epic.title_bonus_percent)}</span>
        <span class="meta-pill">${taskCount} 个 Task</span>
      </div>
      ${
        completed && epic.engraving
          ? `<div class="engraving">“${escapeHtml(epic.engraving)}”</div>`
          : ""
      }
    `;
    host.appendChild(card);
  });
  $$("[data-complete-epic]").forEach((button) => {
    button.addEventListener("click", () => openCompleteEpicModal(button.dataset.completeEpic));
  });
  $$("[data-edit-epic]").forEach((button) => {
    button.addEventListener("click", () => openEpicModal(button.dataset.editEpic));
  });
}

function epicMatchesQuery(epic, query) {
  if (!query) return true;
  const haystack = [
    epic.title,
    epic.description,
    epic.main_dimension,
    dimensionName(epic.main_dimension),
    epic.status === "completed" ? "已结项" : "进行中",
    epic.engraving,
  ]
    .join(" ")
    .toLowerCase();
  return haystack.includes(query);
}

function renderTasks() {
  const host = $("#taskList");
  host.innerHTML = "";
  const allTasks = Object.values(state.tasks).filter(
    (task) => !task.deleted && task.status !== "deleted"
  );
  const tasks = allTasks
    .filter((task) => {
      if (!showCompletedTasks && task.status === "completed") return false;
      return taskMatchesQuery(task, taskSearchQuery);
    })
    .sort((a, b) => {
      if (a.status !== b.status) return a.status === "active" ? -1 : 1;
      return String(b.created_at).localeCompare(String(a.created_at));
    });
  const count = $("#taskCount");
  if (count) {
    count.textContent = taskSearchQuery || !showCompletedTasks
      ? `${tasks.length} / ${allTasks.length} 个 Task`
      : `${allTasks.length} 个 Task`;
  }
  if (!tasks.length) {
    if (!showCompletedTasks && !taskSearchQuery) {
      host.innerHTML = '<p class="muted">当前没有未完成的 Task。勾选「显示已完成」可以查看全部记录。</p>';
    } else if (taskSearchQuery) {
      host.innerHTML = '<p class="muted">没有匹配的 Task。试试搜索名称、属性、数值或里程碑。</p>';
    } else {
      host.innerHTML = '<p class="muted">暂无 Task。点击「新建 Task」，可同时选择多个属性并设置增益或减益。</p>';
    }
    return;
  }
  tasks.forEach((task) => {
    const epic = state.epics[task.epic_id];
    const effects = taskEffects(task);
    const completed = task.status === "completed";
    const repeatable = Boolean(task.repeatable);
    const times = Number(task.times_completed || 0);
    const sanDelta = effects
      .filter((effect) => effect.dimension === "san")
      .reduce((total, effect) => total + Number(effect.delta || 0), 0);
    const sanInsufficient = sanDelta < 0 && Number(state.derived.san) + sanDelta < 0;
    const row = document.createElement("div");
    row.className = `task-row ${completed ? "completed" : ""}`;
    row.innerHTML = `
      <button class="task-check" data-complete-task="${task.id}" ${(completed && !repeatable) || sanInsufficient ? "disabled" : ""} title="${sanInsufficient ? `SAN 不足：需要 ${Math.abs(sanDelta)}` : ""}">${repeatable ? "＋" : "✓"}</button>
      <div>
        <strong>${escapeHtml(task.title)}</strong>
        <p class="muted">
          ${escapeHtml(epic?.title || "独立 Task")}
          ${repeatable ? ` · 可重复${times ? `（已记录 ${times} 次）` : ""}` : ""}
        </p>
      </div>
      <span class="task-value">
        ${effects.map((effect) => `<span class="effect-chip">${dimensionName(effect.dimension)} ${formatDelta(effect.delta)}</span>`).join("")}
        ${sanInsufficient ? '<span class="warning-chip">SAN 不足</span>' : ""}
      </span>
      <div class="task-actions">
        <button class="small-button" type="button" data-edit-task="${task.id}">编辑</button>
        <button class="icon-button" type="button" data-delete-task="${task.id}" title="删除 Task">🗑</button>
      </div>
    `;
    host.appendChild(row);
  });
  $$("[data-complete-task]").forEach((button) => {
    button.addEventListener("click", () => completeTask(button.dataset.completeTask));
  });
  $$("[data-edit-task]").forEach((button) => {
    button.addEventListener("click", () => openTaskModal(button.dataset.editTask));
  });
  $$("[data-delete-task]").forEach((button) => {
    button.addEventListener("click", () => deleteTask(button.dataset.deleteTask));
  });
}

function taskMatchesQuery(task, query) {
  if (!query) return true;
  const epic = state.epics[task.epic_id];
  const effects = taskEffects(task)
    .map((effect) => `${effect.dimension} ${dimensionName(effect.dimension)} ${effect.delta} ${formatDelta(effect.delta)}`)
    .join(" ");
  const haystack = [
    task.title,
    epic?.title || "独立 Task",
    effects,
    task.repeatable ? "可重复" : "一次性",
    task.status === "completed" ? "已完成" : "进行中",
  ]
    .join(" ")
    .toLowerCase();
  return haystack.includes(query);
}

function renderTitles() {
  const slotHost = $("#equipmentSlots");
  slotHost.innerHTML = "";
  const equipped = state.equipped || [];
  const titles = Object.values(state.titles || {});
  for (let index = 0; index < 3; index += 1) {
    const titleId = equipped[index];
    const title = titleId ? state.titles?.[titleId] : null;
    const slot = document.createElement("div");
    slot.className = `equipment-slot ${title ? "filled" : ""}`;
    if (title) {
      slot.innerHTML = `<div><div class="emoji">${title.emoji}</div><div class="name">${escapeHtml(title.name)}</div></div>`;
      slot.title = "点击卸下";
      slot.addEventListener("click", () => unequipTitle(titleId));
    } else {
      slot.innerHTML = `<div><div class="emoji">＋</div><div class="name">空槽位</div></div>`;
    }
    slotHost.appendChild(slot);
  }

  const host = $("#titleLibrary");
  host.innerHTML = "";
  if (!titles.length) {
    host.innerHTML = '<p class="muted">创建里程碑后会生成对应的称号。</p>';
    return;
  }
  titles.forEach((title) => {
    const unlocked = Boolean(title.unlocked);
    const isEquipped = state.equipped.includes(title.id);
    const canEquip = unlocked && !isEquipped && state.equipped.length < 3;
    const item = document.createElement("div");
    item.className = `title-item ${unlocked ? "" : "locked"}`;
    item.innerHTML = `
      <div class="title-top">
        <span class="title-name">${title.emoji} ${escapeHtml(title.name)}</span>
        ${
          isEquipped
            ? '<button class="small-button" data-unequip-title="' + title.id + '">卸下</button>'
            : canEquip
              ? '<button class="small-button" data-equip-title="' + title.id + '">装备</button>'
              : unlocked
                ? '<span class="meta-pill">已装备</span>'
                : '<span class="meta-pill">未解锁</span>'
        }
      </div>
      <p>${escapeHtml(title.description)}</p>
      <p>加成：${dimensionName(title.target_dimension || "professional")} ${formatPercent(title.bonus_percent)}</p>
    `;
    host.appendChild(item);
  });

  $$("[data-equip-title]").forEach((button) => {
    button.addEventListener("click", () => equipTitle(button.dataset.equipTitle));
  });
  $$("[data-unequip-title]").forEach((button) => {
    button.addEventListener("click", () => unequipTitle(button.dataset.unequipTitle));
  });
}

function renderEvents(containerId, limit = 10) {
  const host = document.getElementById(containerId);
  if (!host) return;
  host.innerHTML = "";
  const events = (state.event_stream || [])
    .slice()
    .sort((a, b) => String(b.created_at || "").localeCompare(String(a.created_at || "")))
    .slice(0, limit);
  if (!events.length) {
    host.innerHTML = '<p class="muted">还没有事件。创建第一个 Task，故事就开始生长。</p>';
    return;
  }
  events.forEach((event) => {
    host.appendChild(renderEventItem(event));
  });
}

function renderEventItem(event) {
  const item = document.createElement("div");
  const deleted = Boolean(event.deleted);
  item.className = `event-item ${deleted ? "deleted" : ""}`;
  const eventId = event.event_id;
  const changes = Array.isArray(event.changes) ? event.changes : [];
  const changesHtml = changes.length
    ? changes
        .map(
          (change) =>
            `<span class="effect-chip">${dimensionName(change.dimension)} ${formatDelta(change.delta)}</span>`
        )
        .join("")
    : '<span class="muted">无属性变化</span>';
  const detailTags = event.details?.tags?.length
    ? `<p class="event-detail">${escapeHtml(event.details.tags.join(" / "))}</p>`
    : "";
  const engraving = event.details?.engraving
    ? `<p class="event-detail">“${escapeHtml(event.details.engraving)}”</p>`
    : "";
  const deleteButton =
    eventId && !deleted
      ? `<button class="icon-button" type="button" data-delete-event="${escapeHtml(eventId)}" title="标记删除">🗑</button>`
      : "";
  const icon = deleted ? "🗑️" : eventIcon(event.type);
  const typeLabel = deleted ? "deleted" : event.type;
  item.innerHTML = `
    <div class="event-icon ${deleted ? "deleted-icon" : ""}">${icon}</div>
    <div class="event-main">
      <div class="event-title-row">
        <strong>${escapeHtml(event.name || event.type)}</strong>
        <span class="event-type ${deleted ? "deleted-type" : ""}">${escapeHtml(typeLabel || "")}</span>
      </div>
      <div class="event-changes">${changesHtml}</div>
      ${detailTags}
      ${engraving}
      <p class="event-time">创建于 ${formatDateTime(event.created_at)}${deleted ? ` · 删除于 ${formatDateTime(event.deleted_at)}` : ""}</p>
    </div>
    <div class="event-actions">${deleteButton}</div>
  `;
  item.querySelectorAll("[data-delete-event]").forEach((button) => {
    button.addEventListener("click", () => deleteEvent(button.dataset.deleteEvent));
  });
  return item;
}

function eventIcon(type) {
  return {
    TASK_CREATED: "🧩",
    TASK_COMPLETED: "✅",
    TASK_UPDATED: "✏️",
    TASK_DELETED: "🗑️",
    EPIC_CREATED: "🎯",
    EPIC_UPDATED: "✏️",
    EPIC_COMPLETED: "🏆",
    ACTION_LOGGED: "📜",
    TITLE_EQUIPPED: "🏅",
    TITLE_UNEQUIPPED: "↩️",
    PROFILE_AWAKENED: "🌱",
    SYSTEM_DAILY_TICK: "⏱️",
    SYSTEM_INIT: "⚙️",
  }[type] || "•";
}

function openEpicModal(epicId = null) {
  const epic = epicId ? state.epics[epicId] : null;
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

function openTaskModal(taskId = null) {
  const task = taskId ? state.tasks[taskId] : null;
  if (taskId && !task) return;
  const existingEffects = new Map(
    (task ? taskEffects(task) : []).map((effect) => [effect.dimension, Number(effect.delta)])
  );
  const allEpics = Object.values(state.epics);
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

function openCompleteEpicModal(epicId) {
  const epic = state.epics[epicId];
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

async function handleModalSubmit(event) {
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

async function completeTask(taskId) {
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

async function deleteTask(taskId) {
  if (!taskId) return;
  const task = state.tasks[taskId];
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

async function deleteEvent(eventId) {
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

async function equipTitle(titleId) {
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

async function unequipTitle(titleId) {
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

async function awaken() {
  try {
    await api("/api/awaken", { method: "POST", body: JSON.stringify({}) });
    toast("八维雷达已觉醒。");
    await loadState();
  } catch (error) {
    toast(error.message, true);
  }
}

function openModal(title, bodyHtml) {
  $("#modalTitle").textContent = title;
  $("#modalBody").innerHTML = bodyHtml;
  $("#modal").hidden = false;
}

function closeModal() {
  $("#modal").hidden = true;
  $("#modalBody").innerHTML = "";
}

function toast(message, isError = false) {
  const element = $("#toast");
  element.textContent = message;
  element.className = `toast${isError ? " error" : ""}`;
  element.hidden = false;
  window.clearTimeout(toast._timer);
  toast._timer = window.setTimeout(() => {
    element.hidden = true;
  }, 3600);
}

function clamp(value, min, max) {
  return Math.min(max, Math.max(min, value));
}

function formatDelta(value) {
  const number = Number(value || 0);
  return `${number >= 0 ? "+" : ""}${number.toLocaleString("zh-CN", { maximumFractionDigits: 1 })}`;
}

function formatPercent(value) {
  const number = Number(value || 0);
  return `${number >= 0 ? "+" : ""}${number.toLocaleString("zh-CN", { maximumFractionDigits: 2 })}%`;
}

function formatDateTime(value) {
  if (!value) return "未知时间";
  const date = new Date(value);
  if (Number.isNaN(date.getTime())) return String(value);
  return date.toLocaleString("zh-CN", {
    year: "numeric",
    month: "2-digit",
    day: "2-digit",
    hour: "2-digit",
    minute: "2-digit",
    second: "2-digit",
    hour12: false,
  });
}

function escapeHtml(value) {
  return String(value ?? "")
    .replaceAll("&", "&amp;")
    .replaceAll("<", "&lt;")
    .replaceAll(">", "&gt;")
    .replaceAll('"', "&quot;")
    .replaceAll("'", "&#039;");
}
