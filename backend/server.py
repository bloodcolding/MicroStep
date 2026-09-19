from __future__ import annotations

import json
import os
import threading
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


class RPGRequestHandler(BaseHTTPRequestHandler):
    server_version = "MicroStepRPG/2.0"

    def do_OPTIONS(self) -> None:
        self.send_response(HTTPStatus.NO_CONTENT)
        self._send_cors()
        self.end_headers()

    def do_GET(self) -> None:
        parsed = urlparse(self.path)
        if parsed.path == "/api/state":
            self._json_response(store.get_state())
            return
        if parsed.path == "/api/events":
            self._json_response({"events": store.read_events()})
            return
        if parsed.path == "/api/meta":
            self._json_response(
                {
                    "dimensions": DIMENSION_META,
                    "pool_dimensions": POOL_DIMENSIONS,
                    "effect_dimensions": TASK_EFFECT_DIMENSIONS,
                    "title_bonus_dimensions": TITLE_BONUS_DIMENSIONS,
                    "max_equipped_titles": MAX_EQUIPPED_TITLES,
                }
            )
            return
        self._serve_static(parsed.path)

    def do_POST(self) -> None:
        parsed = urlparse(self.path)
        body = self._read_json_body()
        try:
            if parsed.path == "/api/epics":
                title = str(body.get("title", "")).strip()
                if not title:
                    raise ValueError("请填写 Epic 标题")
                main_dimension = str(body.get("main_dimension") or "professional")
                if main_dimension not in DIMENSION_META or main_dimension == "san":
                    raise ValueError("主维度不合法")
                title_bonus_dimension = str(body.get("title_bonus_dimension") or "professional")
                if title_bonus_dimension not in TITLE_BONUS_DIMENSIONS:
                    raise ValueError("称号加成属性不合法")
                title_bonus_percent = float(body.get("title_bonus_percent") or 0)
                event = store.create_epic(
                    title=title,
                    description=str(body.get("description", "")).strip(),
                    main_dimension=main_dimension,
                    title_bonus_dimension=title_bonus_dimension,
                    title_bonus_percent=title_bonus_percent,
                    title_emoji=str(body.get("title_emoji") or "🏅"),
                )
                self._json_response({"ok": True, "event": event, "state": store.get_state()})
                return

            if parsed.path.startswith("/api/epics/") and (
                parsed.path.endswith("/complete") or parsed.path.endswith("/update")
            ):
                parts = parsed.path.rstrip("/").split("/")
                if len(parts) >= 4 and parts[-1] in ("complete", "update"):
                    epic_id = parts[-2]
                    if parts[-1] == "complete":
                        engraving = str(body.get("engraving", "")).strip()
                        if len(engraving) < 2:
                            raise ValueError("请写下 1-2 句结项铭文")
                        event = store.complete_epic(epic_id, engraving)
                        self._json_response({"ok": True, "event": event, "state": store.get_state()})
                        return
                    if parts[-1] == "update":
                        main_dimension = str(body.get("main_dimension") or "professional")
                        if main_dimension not in DIMENSION_META or main_dimension == "san":
                            raise ValueError("主维度不合法")
                        title_bonus_dimension = str(body.get("title_bonus_dimension") or "professional")
                        if title_bonus_dimension not in TITLE_BONUS_DIMENSIONS:
                            raise ValueError("称号加成属性不合法")
                        event = store.update_epic(
                            epic_id,
                            title=str(body.get("title", "")),
                            description=str(body.get("description", "")),
                            main_dimension=main_dimension,
                            title_bonus_dimension=title_bonus_dimension,
                            title_bonus_percent=float(body.get("title_bonus_percent") or 0),
                            title_emoji=str(body.get("title_emoji") or "🏅"),
                        )
                        self._json_response({"ok": True, "event": event, "state": store.get_state()})
                        return

            if parsed.path == "/api/tasks":
                title = str(body.get("title", "")).strip()
                if not title:
                    raise ValueError("请填写 Task 标题")
                epic_id = _clean_id(body.get("epic_id"))
                effects = body.get("effects") if isinstance(body.get("effects"), list) else None
                repeatable = bool(body.get("repeatable"))
                tags = body.get("tags") if isinstance(body.get("tags"), list) else []
                event = store.create_task(
                    title,
                    epic_id,
                    effects=effects,
                    repeatable=repeatable,
                    tags=tags,
                )
                self._json_response({"ok": True, "event": event, "state": store.get_state()})
                return

            if parsed.path.startswith("/api/tasks/"):
                parts = parsed.path.rstrip("/").split("/")
                if len(parts) >= 4:
                    task_id = parts[-2]
                    action = parts[-1]
                    if action in ("complete", "log"):
                        event = store.complete_task(task_id, str(body.get("note", "")))
                        self._json_response({"ok": True, "event": event, "state": store.get_state()})
                        return
                    if action == "update":
                        effects = body.get("effects") if isinstance(body.get("effects"), list) else None
                        event = store.update_task(
                            task_id,
                            title=str(body.get("title", "")).strip() or None,
                            epic_id=_clean_id(body.get("epic_id")),
                            effects=effects,
                            repeatable=bool(body.get("repeatable")) if "repeatable" in body else None,
                        )
                        self._json_response({"ok": True, "event": event, "state": store.get_state()})
                        return
                    if action == "delete":
                        event = store.delete_task(task_id)
                        self._json_response({"ok": True, "event": event, "state": store.get_state()})
                        return
                    self._json_response({"ok": False, "error": "Task 操作不存在"}, status=HTTPStatus.NOT_FOUND)
                    return

            if parsed.path.startswith("/api/events/") and parsed.path.endswith("/delete"):
                parts = parsed.path.rstrip("/").split("/")
                if len(parts) >= 4 and parts[-1] == "delete":
                    event_id = parts[-2]
                    deletion = store.delete_event(event_id, str(body.get("note", "")))
                    self._json_response({"ok": True, "event": deletion, "state": store.get_state()})
                    return

            if parsed.path == "/api/titles/equip":
                title_id = _clean_id(body.get("title_id"))
                event = store.equip_title(title_id)
                self._json_response({"ok": True, "event": event, "state": store.get_state()})
                return

            if parsed.path == "/api/titles/unequip":
                title_id = _clean_id(body.get("title_id"))
                event = store.unequip_title(title_id)
                self._json_response({"ok": True, "event": event, "state": store.get_state()})
                return

            if parsed.path == "/api/awaken":
                event = store.awaken()
                self._json_response({"ok": True, "event": event, "state": store.get_state()})
                return

            if parsed.path == "/api/system/tick":
                store.manual_tick()
                self._json_response({"ok": True, "state": store.get_state()})
                return

            self._json_response({"ok": False, "error": "接口不存在"}, status=HTTPStatus.NOT_FOUND)
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
