// 后端 API 访问层：将原 HTTP 路由映射为 Tauri v2 IPC invoke 调用。

const { invoke } = window.__TAURI__.core;

// 精确路由表：路径 → IPC 命令名。
const exactRoutes = {
  "/api/state": "get_state",
  "/api/meta": "get_meta",
  "/api/epics": "create_epic",
  "/api/tasks": "create_task",
  "/api/titles/equip": "equip_title",
  "/api/titles/unequip": "unequip_title",
  "/api/awaken": "awaken",
  "/api/system/tick": "system_tick",
};

// 参数路由表：null 表示捕获段，paramName 为其 invoke 参数名。
const paramRoutes = [
  { segments: ["api", "epics", null, "update"], command: "update_epic", paramName: "epic_id" },
  { segments: ["api", "epics", null, "complete"], command: "complete_epic", paramName: "epic_id" },
  { segments: ["api", "tasks", null, "log"], command: "log_task", paramName: "task_id" },
  { segments: ["api", "tasks", null, "complete"], command: "log_task", paramName: "task_id" },
  { segments: ["api", "tasks", null, "update"], command: "update_task", paramName: "task_id" },
  { segments: ["api", "tasks", null, "delete"], command: "delete_task", paramName: "task_id" },
  { segments: ["api", "events", null, "delete"], command: "delete_event", paramName: "event_id" },
];

// 先查精确表，再按段数与字面段匹配参数路由；捕获段做 decodeURIComponent。
function matchRoute(path) {
  if (path in exactRoutes) return { command: exactRoutes[path], pathParams: {} };
  const parts = path.split("/").filter(Boolean);
  for (const route of paramRoutes) {
    if (route.segments.length !== parts.length) continue;
    let captured = null;
    let matched = true;
    for (let i = 0; i < parts.length; i++) {
      if (route.segments[i] === null) captured = parts[i];
      else if (route.segments[i] !== parts[i]) { matched = false; break; }
    }
    if (matched) {
      return { command: route.command, pathParams: { [route.paramName]: decodeURIComponent(captured) } };
    }
  }
  return null;
}

// Tauri v2 命令参数键为 camelCase（如 task_id → taskId）：顶层键统一转换，
// 嵌套值（effects[].dimension 等）原样透传，不做递归。
const camelKey = (key) => key.replace(/_([a-z])/g, (_, ch) => ch.toUpperCase());

export async function api(path, options = {}) {
  const route = matchRoute(path);
  if (!route) throw new Error("接口不存在");
  const body = options.body ? JSON.parse(options.body) : {};
  const args = Object.fromEntries(
    Object.entries({ ...route.pathParams, ...body }).map(([key, value]) => [camelKey(key), value])
  );
  const payload = await invoke(route.command, args);
  if (payload && payload.ok === false) {
    throw new Error(payload.error || "请求失败");
  }
  return payload;
}

export async function fetchMeta() {
  return api("/api/meta");
}

export async function fetchState() {
  return api("/api/state");
}
