// 设置面板（add-settings-panel）：顶栏入口、三分类 tab 与只读信息渲染。

import { t, onLocaleChanged } from "./i18n/index.js";
import { $, $$, escapeHtml, formatDateTime } from "./utils.js";
import { getState } from "./state.js";
import { openModal } from "./ui.js";
import {
  bindSyncPanel,
  refreshSyncPanel,
  setSyncPanelRefreshListener,
  syncFormHtml,
  snapshotSyncForm,
  applySyncFormSnapshot,
  renderSyncEcho,
} from "./sync.js";

const settingsTabIds = ["sync", "archive", "about"];

let latestSyncConfig = null;
let settingsOpen = false;
let activeSettingsTab = "sync";

function openSettings() {
  openModal(t("settings.title"), settingsPanelHtml());
  settingsOpen = true;
  activeSettingsTab = "sync";
  latestSyncConfig = null;
  $("#modal .modal-card").classList.add("settings-modal");
  bindSyncPanel();
  bindSettingsTabs();
  selectSettingsTab(activeSettingsTab);
  renderDataArchive(null);
  refreshSyncPanel();
}

function settingsPanelHtml() {
  const tabs = settingsTabIds
    .map(
      (id) => `
        <button
          id="settingsTab-${id}"
          class="settings-tab"
          type="button"
          data-settings-tab="${id}"
          role="tab"
          aria-selected="false"
          aria-controls="settingsPanel-${id}"
        >${t(`settings.tab.${id}`)}</button>`
    )
    .join("");
  return `
    <div class="settings-panel">
      <div class="settings-tabs" role="tablist" aria-label="${t("settings.tabsAria")}">${tabs}</div>
      <div id="settingsPanel-sync" class="settings-tab-panel" role="tabpanel" aria-labelledby="settingsTab-sync" hidden>
        ${syncFormHtml()}
      </div>
      <div id="settingsPanel-archive" class="settings-tab-panel" role="tabpanel" aria-labelledby="settingsTab-archive" hidden>
        <div id="settingsArchiveBody"></div>
      </div>
      <div id="settingsPanel-about" class="settings-tab-panel" role="tabpanel" aria-labelledby="settingsTab-about" hidden>
        <div class="settings-about">
          <p>${t("settings.about.lead")}</p>
          <p>${t("settings.about.events")}</p>
        </div>
      </div>
      <div class="form-actions">
        <button class="ghost-button" type="button" data-close-modal>${t("common.cancel")}</button>
      </div>
    </div>
  `;
}

function bindSettingsTabs() {
  $$(".settings-tab").forEach((button) => {
    button.addEventListener("click", () => {
      activeSettingsTab = button.dataset.settingsTab;
      selectSettingsTab(activeSettingsTab);
    });
  });
}

function selectSettingsTab(selectedId) {
  if (selectedId === "archive") renderDataArchive(latestSyncConfig);
  settingsTabIds.forEach((id) => {
    $(`#settingsTab-${id}`).setAttribute("aria-selected", String(id === selectedId));
    $(`#settingsPanel-${id}`).hidden = id !== selectedId;
  });
}

function renderDataArchive(config) {
  const container = $("#settingsArchiveBody");
  if (!container) return;
  const eventCount = (getState()?.event_stream || []).length;
  const lastResult = config ? config.last_result || t("sync.notSyncedYet") : t("sync.readFailed");
  const lastSyncAt = config?.last_sync_at ? formatDateTime(config.last_sync_at) : config ? t("sync.notRecorded") : t("sync.readFailed");
  container.innerHTML = `
    <div class="settings-facts">
      <div class="settings-fact">
        <span>${t("settings.archive.location")}</span>
        <strong><code>%APPDATA%\\com.microstep.app</code></strong>
      </div>
      <div class="settings-fact">
        <span>${t("settings.archive.eventCount")}</span>
        <strong>${eventCount}</strong>
      </div>
      <div class="settings-fact">
        <span>${t("settings.archive.lastSyncAt")}</span>
        <strong>${escapeHtml(lastSyncAt)}</strong>
      </div>
      <div class="settings-fact">
        <span>${t("settings.archive.lastResult")}</span>
        <strong>${escapeHtml(lastResult)}</strong>
      </div>
    </div>
    <p class="muted">${t("settings.archive.note")}</p>
  `;
}

// 语言切换时若设置面板打开：按新语言重建面板，保持当前 tab 与未提交的表单值。
onLocaleChanged(() => {
  if (!settingsOpen || !$("#settingsPanel-sync")) return;
  const snapshot = snapshotSyncForm();
  $("#modalTitle").textContent = t("settings.title");
  $("#modalBody").innerHTML = settingsPanelHtml();
  $("#modal .modal-card").classList.add("settings-modal");
  bindSyncPanel();
  bindSettingsTabs();
  selectSettingsTab(activeSettingsTab);
  renderDataArchive(latestSyncConfig);
  renderSyncEcho(latestSyncConfig);
  applySyncFormSnapshot(snapshot);
});

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
        settingsOpen = false;
      }
    },
    true
  );
});
