from __future__ import annotations

import json
import os
import threading
from collections.abc import Callable
from http import HTTPStatus
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path
from typing import Any
from urllib.parse import urlparse

from .domain import (
    DIMENSION_META,
    MAX_EQUIPPED_TITLES,
    POOL_DIMENSIONS,
    TASK_EFFECT_DIMENSIONS,
    TITLE_BONUS_DIMENSIONS,
)
from .store import EventStore


ROOT = Path(__file__).resolve().parent.parent
FRONTEND_DIR = ROOT / "frontend"
DATA_DIR = ROOT / "data"
EVENT_PATH = Path(os.environ.get("MICROSTEP_EVENT_PATH") or (DATA_DIR / "events.jsonl"))

# 延迟到 main() 创建：避免 import backend.server 就在磁盘上创建数据文件。
store: EventStore | None = None


# ---------------------------------------------------------------------------
# API handlers：统一签名 (store, body, params) -> 响应 payload。
# 抛 KeyError/ValueError 由分发层转 400，其余异常转 500。
# ---------------------------------------------------------------------------

ApiHandler = Callable[[EventStore, dict[str, Any], dict[str, str]], dict[str, Any]]

GetApiHandler = Callable[[EventStore], Any]


def _event_response(target_store: EventStore, event: dict[str, Any]) -> dict[str, Any]:
    return {"ok": True, "event": event, "state": target_store.get_state()}


def _get_state_payload(target_store: EventStore) -> dict[str, Any]:
    return target_store.get_state()


def _get_meta_payload(target_store: EventStore) -> dict[str, Any]:
    return {
        "dimensions": DIMENSION_META,
        "pool_dimensions": POOL_DIMENSIONS,
        "effect_dimensions": TASK_EFFECT_DIMENSIONS,
        "title_bonus_dimensions": TITLE_BONUS_DIMENSIONS,
        "max_equipped_titles": MAX_EQUIPPED_TITLES,
    }


def _get_events_payload(target_store: EventStore) -> dict[str, Any]:
    return {"events": target_store.read_events()}


def _post_create_epic(target_store: EventStore, body: dict[str, Any], params: dict[str, str]) -> dict[str, Any]:
    title = str(body.get("title", "")).strip()
    if not title:
        raise ValueError("请填写 Epic 标题")
    main_dimension = str(body.get("main_dimension") or "professional")
    if main_dimension not in DIMENSION_META or main_dimension == "san":
        raise ValueError("主维度不合法")
    title_bonus_dimension = str(body.get("title_bonus_dimension") or "professional")
    if title_bonus_dimension not in TITLE_BONUS_DIMENSIONS:
        raise ValueError("称号加成属性不合法")
    event = target_store.create_epic(
        title=title,
        description=str(body.get("description", "")).strip(),
        main_dimension=main_dimension,
        title_bonus_dimension=title_bonus_dimension,
        title_bonus_percent=float(body.get("title_bonus_percent") or 0),
        title_emoji=str(body.get("title_emoji") or "🏅"),
    )
    return _event_response(target_store, event)


def _post_update_epic(target_store: EventStore, body: dict[str, Any], params: dict[str, str]) -> dict[str, Any]:
    main_dimension = str(body.get("main_dimension") or "professional")
    if main_dimension not in DIMENSION_META or main_dimension == "san":
        raise ValueError("主维度不合法")
    title_bonus_dimension = str(body.get("title_bonus_dimension") or "professional")
    if title_bonus_dimension not in TITLE_BONUS_DIMENSIONS:
        raise ValueError("称号加成属性不合法")
    event = target_store.update_epic(
        params["epic_id"],
        title=str(body.get("title", "")),
        description=str(body.get("description", "")),
        main_dimension=main_dimension,
        title_bonus_dimension=title_bonus_dimension,
        title_bonus_percent=float(body.get("title_bonus_percent") or 0),
        title_emoji=str(body.get("title_emoji") or "🏅"),
    )
    return _event_response(target_store, event)


def _post_complete_epic(target_store: EventStore, body: dict[str, Any], params: dict[str, str]) -> dict[str, Any]:
    engraving = str(body.get("engraving", "")).strip()
    if len(engraving) < 2:
        raise ValueError("请写下 1-2 句结项铭文")
    event = target_store.complete_epic(params["epic_id"], engraving)
    return _event_response(target_store, event)


def _post_create_task(target_store: EventStore, body: dict[str, Any], params: dict[str, str]) -> dict[str, Any]:
    title = str(body.get("title", "")).strip()
    if not title:
        raise ValueError("请填写 Task 标题")
    event = target_store.create_task(
        title,
        _clean_id(body.get("epic_id")),
        effects=body.get("effects") if isinstance(body.get("effects"), list) else None,
        repeatable=bool(body.get("repeatable")),
        tags=body.get("tags") if isinstance(body.get("tags"), list) else [],
    )
    return _event_response(target_store, event)


def _post_log_task(target_store: EventStore, body: dict[str, Any], params: dict[str, str]) -> dict[str, Any]:
    """记录一次 Task；/complete 为兼容旧接口的别名路由。"""
    event = target_store.complete_task(params["task_id"], str(body.get("note", "")))
    return _event_response(target_store, event)


def _post_update_task(target_store: EventStore, body: dict[str, Any], params: dict[str, str]) -> dict[str, Any]:
    event = target_store.update_task(
        params["task_id"],
        title=str(body.get("title", "")).strip() or None,
        epic_id=_clean_id(body.get("epic_id")),
        effects=body.get("effects") if isinstance(body.get("effects"), list) else None,
        repeatable=bool(body.get("repeatable")) if "repeatable" in body else None,
    )
    return _event_response(target_store, event)


def _post_delete_task(target_store: EventStore, body: dict[str, Any], params: dict[str, str]) -> dict[str, Any]:
    event = target_store.delete_task(params["task_id"])
    return _event_response(target_store, event)


def _post_delete_event(target_store: EventStore, body: dict[str, Any], params: dict[str, str]) -> dict[str, Any]:
    deletion = target_store.delete_event(params["event_id"], str(body.get("note", "")))
    return _event_response(target_store, deletion)


def _post_equip_title(target_store: EventStore, body: dict[str, Any], params: dict[str, str]) -> dict[str, Any]:
    event = target_store.equip_title(_clean_id(body.get("title_id")) or "")
    return _event_response(target_store, event)


def _post_unequip_title(target_store: EventStore, body: dict[str, Any], params: dict[str, str]) -> dict[str, Any]:
    event = target_store.unequip_title(_clean_id(body.get("title_id")) or "")
    return _event_response(target_store, event)


def _post_awaken(target_store: EventStore, body: dict[str, Any], params: dict[str, str]) -> dict[str, Any]:
    event = target_store.awaken()
    return _event_response(target_store, event)


def _post_system_tick(target_store: EventStore, body: dict[str, Any], params: dict[str, str]) -> dict[str, Any]:
    target_store.manual_tick()
    return {"ok": True, "state": target_store.get_state()}


# ---------------------------------------------------------------------------
# 路由表：精确路径查字典，带参数路径按 {param} 段匹配。
# ---------------------------------------------------------------------------

GET_API_ROUTES: dict[str, GetApiHandler] = {
    "/api/state": _get_state_payload,
    "/api/meta": _get_meta_payload,
    "/api/events": _get_events_payload,
}

POST_API_ROUTES: dict[str, ApiHandler] = {
    "/api/epics": _post_create_epic,
    "/api/tasks": _post_create_task,
    "/api/titles/equip": _post_equip_title,
    "/api/titles/unequip": _post_unequip_title,
    "/api/awaken": _post_awaken,
    "/api/system/tick": _post_system_tick,
}

POST_API_PARAM_ROUTES: dict[str, ApiHandler] = {
    "/api/epics/{epic_id}/complete": _post_complete_epic,
    "/api/epics/{epic_id}/update": _post_update_epic,
    "/api/tasks/{task_id}/log": _post_log_task,
    "/api/tasks/{task_id}/complete": _post_log_task,
    "/api/tasks/{task_id}/update": _post_update_task,
    "/api/tasks/{task_id}/delete": _post_delete_task,
    "/api/events/{event_id}/delete": _post_delete_event,
}


def _match_param_route(path: str) -> tuple[ApiHandler, dict[str, str]] | None:
    segments = path.strip("/").split("/")
    for pattern, handler in POST_API_PARAM_ROUTES.items():
        parts = pattern.strip("/").split("/")
        if len(parts) != len(segments):
            continue
        params: dict[str, str] = {}
        for expected, actual in zip(parts, segments):
            if expected.startswith("{") and expected.endswith("}"):
                params[expected[1:-1]] = actual
            elif expected != actual:
                break
        else:
            return handler, params
    return None


class RPGRequestHandler(BaseHTTPRequestHandler):
    server_version = "MicroStepRPG/2.0"

    def do_OPTIONS(self) -> None:
        self.send_response(HTTPStatus.NO_CONTENT)
        self._send_cors()
        self.end_headers()

    def do_GET(self) -> None:
        parsed = urlparse(self.path)
        handler = GET_API_ROUTES.get(parsed.path)
        if handler is not None and store is not None:
            self._json_response(handler(store))
            return
        self._serve_static(parsed.path)

    def do_POST(self) -> None:
        parsed = urlparse(self.path)
        try:
            body = self._read_json_body()
            if store is None:
                raise RuntimeError("服务尚未初始化")
            route = POST_API_ROUTES.get(parsed.path)
            if route is not None:
                self._json_response(route(store, body, {}))
                return
            matched = _match_param_route(parsed.path)
            if matched is None:
                self._json_response({"ok": False, "error": "接口不存在"}, status=HTTPStatus.NOT_FOUND)
                return
            handler, params = matched
            self._json_response(handler(store, body, params))
        except (KeyError, ValueError) as exc:
            self._json_response({"ok": False, "error": str(exc)}, status=HTTPStatus.BAD_REQUEST)
        except Exception as exc:  # pragma: no cover - 保留为可诊断错误
            self._json_response(
                {"ok": False, "error": f"服务器内部错误: {exc}"},
                status=HTTPStatus.INTERNAL_SERVER_ERROR,
            )

    def _read_json_body(self) -> dict[str, Any]:
        length = int(self.headers.get("Content-Length") or 0)
        if length <= 0:
            return {}
        raw = self.rfile.read(length)
        if not raw:
            return {}
        try:
            data = json.loads(raw.decode("utf-8"))
        except json.JSONDecodeError as exc:
            raise ValueError("请求 JSON 格式错误") from exc
        return data if isinstance(data, dict) else {}

    def _json_response(self, payload: Any, status: HTTPStatus = HTTPStatus.OK) -> None:
        data = json.dumps(payload, ensure_ascii=False).encode("utf-8")
        self.send_response(status)
        self._send_cors()
        self.send_header("Content-Type", "application/json; charset=utf-8")
        self.send_header("Content-Length", str(len(data)))
        self.end_headers()
        self.wfile.write(data)

    def _send_cors(self) -> None:
        self.send_header("Access-Control-Allow-Origin", "*")
        self.send_header("Access-Control-Allow-Methods", "GET, POST, OPTIONS")
        self.send_header("Access-Control-Allow-Headers", "Content-Type")

    def _serve_static(self, path: str) -> None:
        if path in ("/", "/index.html"):
            rel = "index.html"
        else:
            rel = path.lstrip("/")
        # 只允许前端目录内的静态资源。
        target = (FRONTEND_DIR / rel).resolve()
        if FRONTEND_DIR.resolve() not in target.parents and target != FRONTEND_DIR.resolve():
            self.send_error(HTTPStatus.NOT_FOUND)
            return
        if not target.is_file():
            self.send_error(HTTPStatus.NOT_FOUND)
            return
        content = target.read_bytes()
        content_type = _content_type(target.suffix)
        self.send_response(HTTPStatus.OK)
        self._send_cors()
        self.send_header("Content-Type", content_type)
        self.send_header("Content-Length", str(len(content)))
        self.end_headers()
        self.wfile.write(content)

    def log_message(self, format: str, *args: Any) -> None:
        print(f"[{self.log_date_time_string()}] {format % args}")


def _clean_id(value: Any) -> str | None:
    if value is None:
        return None
    text = str(value).strip()
    return text or None


def _content_type(suffix: str) -> str:
    return {
        ".html": "text/html; charset=utf-8",
        ".css": "text/css; charset=utf-8",
        ".js": "application/javascript; charset=utf-8",
        ".json": "application/json; charset=utf-8",
        ".svg": "image/svg+xml",
        ".png": "image/png",
        ".jpg": "image/jpeg",
        ".ico": "image/x-icon",
    }.get(suffix, "application/octet-stream")


class Ticker:
    def __init__(self, store: EventStore, interval_seconds: int = 600) -> None:
        self.store = store
        self.interval = interval_seconds
        self._stop = threading.Event()

    def start(self) -> None:
        thread = threading.Thread(target=self._run, name="daily-tick", daemon=True)
        thread.start()

    def _run(self) -> None:
        while not self._stop.is_set():
            try:
                self.store.ensure_daily_ticks()
            except Exception as exc:  # pragma: no cover
                print(f"Daily tick error: {exc}")
            self._stop.wait(self.interval)

    def stop(self) -> None:
        self._stop.set()


def main(host: str = "127.0.0.1", port: int = 8765, event_path: str | Path | None = None) -> None:
    global store
    store = EventStore(Path(event_path) if event_path is not None else EVENT_PATH)
    store.ensure_initialized()
    store.ensure_daily_ticks()
    ticker = Ticker(store)
    ticker.start()
    httpd = ThreadingHTTPServer((host, port), RPGRequestHandler)
    print(f"🌱 MicroStep RPG 已启动: http://{host}:{port}")
    print(f"事件流文件: {store.path}")
    try:
        httpd.serve_forever()
    except KeyboardInterrupt:
        print("\n正在停止服务...")
    finally:
        ticker.stop()
        httpd.server_close()


if __name__ == "__main__":
    import argparse

    parser = argparse.ArgumentParser(description="MicroStep RPG local-first server")
    parser.add_argument("--host", default="127.0.0.1")
    parser.add_argument("--port", type=int, default=8765)
    args = parser.parse_args()
    main(args.host, args.port)
