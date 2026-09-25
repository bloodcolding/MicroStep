// 设置面板（add-settings-panel）：顶栏入口、三分类 tab 与只读信息渲染。

import { $, $$, escapeHtml, formatDateTime } from "./utils.js";
import { getState } from "./state.js";
import { openModal } from "./ui.js";
import { bindSyncPanel, refreshSyncPanel, setSyncPanelRefreshListener, syncFormHtml } from "./sync.js";

const settingsTabs = [
  { id: "sync", label: "☁️ 代码仓同步" },
  { id: "archive", label: "📦 数据档案" },
  { id: "about", label: "ℹ️ 关于" },
];

let latestSyncConfig = null;

function openSettings() {
  openModal("设置", settingsPanelHtml());
  latestSyncConfig = null;
  $("#modal .modal-card").classList.add("settings-modal");
  bindSyncPanel();
  bindSettingsTabs();
  selectSettingsTab("sync");
  renderDataArchive(null);
  refreshSyncPanel();
}

function settingsPanelHtml() {
  const tabs = settingsTabs
    .map(
      (tab) => `
        <button
          id="settingsTab-${tab.id}"
          class="settings-tab"
          type="button"
          data-settings-tab="${tab.id}"
          role="tab"
          aria-selected="false"
          aria-controls="settingsPanel-${tab.id}"
        >${tab.label}</button>`
    )
    .join("");
  return `
    <div class="settings-panel">
      <div class="settings-tabs" role="tablist" aria-label="设置分类">${tabs}</div>
      <div id="settingsPanel-sync" class="settings-tab-panel" role="tabpanel" aria-labelledby="settingsTab-sync" hidden>
        ${syncFormHtml}
      </div>
      <div id="settingsPanel-archive" class="settings-tab-panel" role="tabpanel" aria-labelledby="settingsTab-archive" hidden>
        <div id="settingsArchiveBody"></div>
      </div>
      <div id="settingsPanel-about" class="settings-tab-panel" role="tabpanel" aria-labelledby="settingsTab-about" hidden>
        <div class="settings-about">
          <p><strong>MicroStep</strong> 是反内卷的个人成长 RPG：自己定义 Task 与属性效果，系统只负责如实结算。</p>
          <p>成长记录采用事件溯源机制：所有变化先追加为不可变事件，当前属性、Task 与称号都由事件流重放得到。</p>
        </div>
      </div>
      <div class="form-actions">
        <button class="ghost-button" type="button" data-close-modal>取消</button>
      </div>
    </div>
  `;
}

function bindSettingsTabs() {
  $$(".settings-tab").forEach((button) => {
    button.addEventListener("click", () => selectSettingsTab(button.dataset.settingsTab));
  });
}

function selectSettingsTab(selectedId) {
  if (selectedId === "archive") renderDataArchive(latestSyncConfig);
  settingsTabs.forEach(({ id }) => {
    $(`#settingsTab-${id}`).setAttribute("aria-selected", String(id === selectedId));
    $(`#settingsPanel-${id}`).hidden = id !== selectedId;
  });
}

function renderDataArchive(config) {
  const container = $("#settingsArchiveBody");
  if (!container) return;
  const eventCount = (getState()?.event_stream || []).length;
  const lastResult = config ? config.last_result || "尚未同步" : "读取失败";
  const lastSyncAt = config?.last_sync_at ? formatDateTime(config.last_sync_at) : config ? "尚未记录" : "读取失败";
  container.innerHTML = `
    <div class="settings-facts">
      <div class="settings-fact">
        <span>数据落点</span>
        <strong><code>%APPDATA%\\com.microstep.app</code></strong>
      </div>
      <div class="settings-fact">
        <span>事件总数</span>
        <strong>${eventCount}</strong>
      </div>
      <div class="settings-fact">
        <span>最近同步时间</span>
        <strong>${escapeHtml(lastSyncAt)}</strong>
      </div>
      <div class="settings-fact">
        <span>最近同步结果</span>
        <strong>${escapeHtml(lastResult)}</strong>
      </div>
    </div>
    <p class="muted">本分类仅查看档案信息；事件流不可在面板中直接修改或删除。</p>
  `;
}

document.addEventListener("DOMContentLoaded", () => {
  $("#settingsBtn").addEventListener("click", openSettings);
  setSyncPanelRefreshListener((config) => {
    latestSyncConfig = config;
    renderDataArchive(config);
  });
  $("#modal").addEventListener(
    "click",
    (event) => {
      if (event.target.closest("[data-close-modal]")) {
        $("#modal .modal-card").classList.remove("settings-modal");
      }
    },
    true
  );
});
