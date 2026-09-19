// 后端 API 访问层：统一 fetch、错误抛出与响应解析。

export async function api(path, options = {}) {
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

export async function fetchMeta() {
  return api("/api/meta");
}

export async function fetchState() {
  return api("/api/state");
}
