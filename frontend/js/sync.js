// 远端同步表单（add-settings-panel）：导出片段与行为，由设置面板挂载。
// 经 api.js 既有收口调用同步 command，保存 / PAT 语义与原同步卡保持一致。

import { fetchSyncConfig, saveSyncConfig, syncNow } from "./api.js";
import { loadState } from "./controller.js";
import { toast } from "./ui.js";
import { $ } from "./utils.js";

let syncPanelRefreshListener = null;

// 同步表单片段：由设置面板嵌入「代码仓同步」分类。
export const syncFormHtml = `
  <div class="settings-sync">
    <p class="muted">事件流通过 Git 远端仓库跨设备同步。PAT 留空保存表示保持已保存值不变，勾选清除才发送空串。</p>
    <div class="form-grid">
      <div class="form-field">
        <label for="syncRemoteUrl">远端地址</label>
        <input id="syncRemoteUrl" type="text" placeholder="远端地址（HTTPS git URL）" />
      </div>
      <div class="form-field">
        <label for="syncPat">PAT</label>
        <input id="syncPat" type="password" placeholder="PAT（Personal Access Token，可留空）" />
      </div>
      <div class="form-field">
        <label for="syncBranch">分支</label>
        <input id="syncBranch" type="text" placeholder="分支（默认 main）" />
      </div>
      <div class="form-field">
        <label class="filter-toggle" for="syncClearPat">
          <input id="syncClearPat" type="checkbox" />
          <span>清除已保存 PAT</span>
        </label>
      </div>
      <div class="form-actions">
        <button id="syncSaveBtn" class="ghost-button" type="button">保存配置</button>
        <button id="syncNowBtn" class="primary-button" type="button">立即同步</button>
      </div>
      <p id="syncStatus" class="muted"></p>
    </div>
  </div>
`;

// 绑定当前面板中的同步按钮；面板 DOM 每次重建后重新调用。
export function bindSyncPanel() {
  $("#syncSaveBtn").addEventListener("click", saveConfig);
  $("#syncNowBtn").addEventListener("click", runSync);
}

// 注册配置刷新回调，供设置面板同步更新数据档案的最近同步信息。
export function setSyncPanelRefreshListener(listener) {
  syncPanelRefreshListener = listener;
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
    $("#syncPat").placeholder = config.pat
      ? `已保存（${config.pat}）· 留空保持不变`
      : "PAT（Personal Access Token，可留空）";
    $("#syncBranch").value = config.branch || "main";
    $("#syncStatus").textContent = config.last_result
      ? `最近同步：${config.last_result}${config.last_sync_at ? ` · ${config.last_sync_at}` : ""}`
      : "尚未同步";
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
    toast("同步配置已保存");
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
    toast(`同步完成：拉取 ${result.pulled} · 推送 ${result.pushed} · 合并 ${result.merged}`);
    await refreshSyncPanel();
    await loadState();
  } catch (error) {
    toast(error.message, true);
    await refreshSyncPanel();
  }
}
