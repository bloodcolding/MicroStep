// 远端同步设置面板（add-git-remote-sync）：配置表单 + 手动同步 + 最近结果展示。
// 经 api.js 既有收口调用同步 command；其余前端模块零改动。

import { fetchSyncConfig, saveSyncConfig, syncNow } from "./api.js";
import { loadState } from "./controller.js";
import { toast } from "./ui.js";
import { $ } from "./utils.js";

async function refreshPanel() {
  try {
    const config = await fetchSyncConfig();
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
  } catch (error) {
    toast(error.message, true);
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
    await refreshPanel();
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
    await refreshPanel();
    await loadState();
  } catch (error) {
    toast(error.message, true);
    await refreshPanel();
  }
}

document.addEventListener("DOMContentLoaded", () => {
  $("#syncSaveBtn").addEventListener("click", saveConfig);
  $("#syncNowBtn").addEventListener("click", runSync);
  refreshPanel();
});
