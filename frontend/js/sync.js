// 远端同步表单（add-settings-panel）：导出片段与行为，由设置面板挂载。
// 经 api.js 既有收口调用同步 command，保存 / PAT 语义与原同步卡保持一致。

import { t } from "./i18n/index.js";
import { fetchSyncConfig, saveSyncConfig, syncNow } from "./api.js";
import { loadState } from "./controller.js";
import { toast } from "./ui.js";
import { $ } from "./utils.js";

let syncPanelRefreshListener = null;

// 同步表单片段（函数形态：语言切换后由设置面板重建并取新文案）。
export function syncFormHtml() {
  return `
  <div class="settings-sync">
    <p class="muted">${t("sync.intro")}</p>
    <div class="form-grid">
      <div class="form-field">
        <label for="syncRemoteUrl">${t("sync.remoteLabel")}</label>
        <input id="syncRemoteUrl" type="text" autocapitalize="off" autocomplete="off" spellcheck="false" placeholder="${t("sync.remotePh")}" />
      </div>
      <div class="form-field">
        <label for="syncPat">PAT</label>
        <input id="syncPat" type="password" autocapitalize="off" autocomplete="off" spellcheck="false" placeholder="${t("sync.patPh")}" />
      </div>
      <div class="form-field">
        <label for="syncBranch">${t("sync.branchLabel")}</label>
        <input id="syncBranch" type="text" autocapitalize="off" autocomplete="off" spellcheck="false" placeholder="${t("sync.branchPh")}" />
      </div>
      <div class="form-field">
        <label class="filter-toggle" for="syncClearPat">
          <input id="syncClearPat" type="checkbox" />
          <span>${t("sync.clearPat")}</span>
        </label>
      </div>
      <div class="form-actions">
        <button id="syncSaveBtn" class="ghost-button" type="button">${t("sync.save")}</button>
        <button id="syncNowBtn" class="primary-button" type="button">${t("sync.now")}</button>
      </div>
      <p id="syncStatus" class="muted"></p>
    </div>
  </div>
`;
}

// 绑定当前面板中的同步按钮；面板 DOM 每次重建后重新调用。
export function bindSyncPanel() {
  $("#syncSaveBtn").addEventListener("click", saveConfig);
  $("#syncNowBtn").addEventListener("click", runSync);
}

// 注册配置刷新回调，供设置面板同步更新数据档案的最近同步信息。
export function setSyncPanelRefreshListener(listener) {
  syncPanelRefreshListener = listener;
}

// 语言切换重建面板用：捕获当前未提交的表单输入。
export function snapshotSyncForm() {
  const remoteInput = $("#syncRemoteUrl");
  if (!remoteInput) return null;
  return {
    remote: remoteInput.value,
    pat: $("#syncPat").value,
    branch: $("#syncBranch").value,
    clearPat: $("#syncClearPat").checked,
  };
}

export function applySyncFormSnapshot(snapshot) {
  if (!snapshot || !$("#syncRemoteUrl")) return;
  $("#syncRemoteUrl").value = snapshot.remote;
  $("#syncPat").value = snapshot.pat;
  $("#syncBranch").value = snapshot.branch;
  $("#syncClearPat").checked = snapshot.clearPat;
}

// 语言切换重建面板用：按已知配置渲染 PAT 占位与最近同步状态（不覆盖输入值）。
export function renderSyncEcho(config) {
  if (!config || !$("#syncRemoteUrl")) return;
  $("#syncPat").placeholder = config.pat
    ? t("sync.patSaved", { pat: config.pat })
    : t("sync.patPh");
  $("#syncStatus").textContent = config.last_result
    ? config.last_sync_at
      ? t("sync.lastResultAt", { result: config.last_result, time: config.last_sync_at })
      : t("sync.lastResult", { result: config.last_result })
    : t("sync.notSyncedYet");
}

// 拉取配置并回显；返回值供数据档案复用最近同步信息。
export async function refreshSyncPanel() {
  try {
    const config = await fetchSyncConfig();
    // 面板可能在请求返回前被关闭；此时无需回显，交给下次打开重新刷新。
    if (!$("#syncRemoteUrl")) return config;
    $("#syncRemoteUrl").value = config.remote_url || "";
    // PAT 脱敏回显：编辑框不回填；留空保存 = 保持不变，勾选清除才发送空串。
    $("#syncPat").value = "";
    $("#syncBranch").value = config.branch || "main";
    renderSyncEcho(config);
    syncPanelRefreshListener?.(config);
    return config;
  } catch (error) {
    toast(error.message, true);
    return null;
  }
}

async function saveConfig() {
  const patInput = $("#syncPat").value.trim();
  const body = {
    remote_url: $("#syncRemoteUrl").value.trim(),
    branch: $("#syncBranch").value.trim() || "main",
  };
  if ($("#syncClearPat").checked) {
    body.pat = "";
  } else if (patInput) {
    body.pat = patInput;
  }
  try {
    await saveSyncConfig(body);
    $("#syncClearPat").checked = false;
    toast(t("toast.syncSaved"));
    await refreshSyncPanel();
    return true;
  } catch (error) {
    toast(error.message, true);
    return false;
  }
}

async function runSync() {
  try {
    // 冒烟发现：填了 PAT 直接点「立即同步」而未先「保存配置」会用旧配置（空 PAT）→ 401。
    // 同步前先自动落盘表单，保存失败则中止。
    if (!(await saveConfig())) return;
    const result = await syncNow();
    toast(t("toast.syncDone", { pulled: result.pulled, pushed: result.pushed, merged: result.merged }));
    await refreshSyncPanel();
    await loadState();
  } catch (error) {
    toast(error.message, true);
    await refreshSyncPanel();
  }
}
