"""server.py HTTP 层测试：起真实 ThreadingHTTPServer 打请求，覆盖路由/校验/静态资源。

运行方式（二选一）：
    python -m unittest tests.test_server -v
    python tests\\test_server.py

使用临时事件流 + 随机端口，不触碰真实数据。
"""

from __future__ import annotations

import json
import sys
import tempfile
import threading
import unittest
import urllib.error
import urllib.request
from http.server import ThreadingHTTPServer
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))

import backend.server as server_module  # noqa: E402
from backend.server import RPGRequestHandler  # noqa: E402
from backend.store import EventStore  # noqa: E402


class QuietHandler(RPGRequestHandler):
    """屏蔽逐请求日志，保持测试输出干净。"""

    def log_message(self, format: str, *args: object) -> None:  # noqa: A002
        pass


class HttpTestCase(unittest.TestCase):
    base_url: str

    @classmethod
    def setUpClass(cls) -> None:
        cls._tmp = tempfile.TemporaryDirectory()
        cls.store = EventStore(Path(cls._tmp.name) / "events.jsonl")
        cls._original_store = server_module.store
        server_module.store = cls.store
        cls.httpd = ThreadingHTTPServer(("127.0.0.1", 0), QuietHandler)
        cls.base_url = f"http://127.0.0.1:{cls.httpd.server_address[1]}"
        cls._thread = threading.Thread(target=cls.httpd.serve_forever, daemon=True)
        cls._thread.start()

    @classmethod
    def tearDownClass(cls) -> None:
        cls.httpd.shutdown()
        cls.httpd.server_close()
        server_module.store = cls._original_store
        cls._tmp.cleanup()

    def get(self, path: str) -> tuple[int, dict]:
        with urllib.request.urlopen(f"{self.base_url}{path}", timeout=5) as response:
            return response.status, json.loads(response.read().decode("utf-8"))

    def post(self, path: str, payload: object) -> tuple[int, dict]:
        request = urllib.request.Request(
            f"{self.base_url}{path}",
            data=json.dumps(payload).encode("utf-8"),
            headers={"Content-Type": "application/json"},
            method="POST",
        )
        try:
            with urllib.request.urlopen(request, timeout=5) as response:
                return response.status, json.loads(response.read().decode("utf-8"))
        except urllib.error.HTTPError as exc:
            return exc.code, json.loads(exc.read().decode("utf-8"))

    def get_raw(self, path: str) -> tuple[int, str]:
        try:
            with urllib.request.urlopen(f"{self.base_url}{path}", timeout=5) as response:
                return response.status, response.read().decode("utf-8")
        except urllib.error.HTTPError as exc:
            return exc.code, exc.read().decode("utf-8", errors="replace")


class GetRoutesTests(HttpTestCase):
    def test_state(self) -> None:
        status, payload = self.get("/api/state")
        self.assertEqual(status, 200)
        self.assertIsNotNone(payload["profile"]["created_at"])
        self.assertIn("epic_infinite_progress", payload["epics"])

    def test_meta_contains_dimensions_and_slots(self) -> None:
        status, payload = self.get("/api/meta")
        self.assertEqual(status, 200)
        self.assertEqual(payload["max_equipped_titles"], 3)
        self.assertIn("san", payload["dimensions"])
        self.assertIn("physical", payload["pool_dimensions"])

    def test_events(self) -> None:
        # 先触发一次 state 初始化（/api/events 只读不初始化）。
        self.get("/api/state")
        status, payload = self.get("/api/events")
        self.assertEqual(status, 200)
        types = [event["type"] for event in payload["events"]]
        self.assertIn("SYSTEM_INIT", types)


class TaskApiTests(HttpTestCase):
    def _create_task(self, **overrides) -> tuple[int, dict]:
        body = {
            "title": "测试 Task",
            "epic_id": "",
            "repeatable": False,
            "effects": [{"dimension": "knowledge", "delta": 5}],
        }
        body.update(overrides)
        return self.post("/api/tasks", body)

    def test_create_and_log_task(self) -> None:
        status, payload = self._create_task()
        self.assertEqual(status, 200)
        self.assertTrue(payload["ok"])
        task_id = payload["event"]["id"]
        status, logged = self.post(f"/api/tasks/{task_id}/log", {"note": ""})
        self.assertEqual(status, 200)
        self.assertTrue(logged["ok"])
        self.assertEqual(logged["state"]["dimensions"]["knowledge"], 5.0)

    def test_legacy_complete_alias(self) -> None:
        _, created = self._create_task(title="旧接口")
        task_id = created["event"]["id"]
        status, payload = self.post(f"/api/tasks/{task_id}/complete", {})
        self.assertEqual(status, 200)
        self.assertTrue(payload["ok"])

    def test_create_task_rejects_empty_title(self) -> None:
        status, payload = self._create_task(title="   ")
        self.assertEqual(status, 400)
        self.assertFalse(payload["ok"])
        self.assertIn("标题", payload["error"])

    def test_create_task_rejects_bad_effects(self) -> None:
        status, payload = self._create_task(effects=[{"dimension": "willpower", "delta": 5}])
        self.assertEqual(status, 400)

    def test_log_task_rejects_insufficient_san(self) -> None:
        _, created = self._create_task(effects=[{"dimension": "san", "delta": -999}])
        status, payload = self.post(f"/api/tasks/{created['event']['id']}/log", {})
        self.assertEqual(status, 400)
        self.assertIn("SAN", payload["error"])

    def test_update_task(self) -> None:
        _, created = self._create_task()
        task_id = created["event"]["id"]
        status, payload = self.post(
            f"/api/tasks/{task_id}/update",
            {"title": "改名", "effects": [{"dimension": "knowledge", "delta": 8}], "repeatable": True},
        )
        self.assertEqual(status, 200)
        task = payload["state"]["tasks"][task_id]
        self.assertEqual(task["title"], "改名")
        self.assertTrue(task["repeatable"])

    def test_delete_task_then_reject_duplicate(self) -> None:
        _, created = self._create_task()
        task_id = created["event"]["id"]
        status, _ = self.post(f"/api/tasks/{task_id}/delete", {})
        self.assertEqual(status, 200)
        status, payload = self.post(f"/api/tasks/{task_id}/delete", {})
        self.assertEqual(status, 400)

    def test_unknown_task_action_404(self) -> None:
        status, payload = self.post("/api/tasks/some-id/unknown", {})
        self.assertEqual(status, 404)
        self.assertFalse(payload["ok"])


class EpicApiTests(HttpTestCase):
    def _create_epic(self, **overrides) -> tuple[int, dict]:
        body = {
            "title": "测试里程碑",
            "description": "",
            "main_dimension": "professional",
            "title_bonus_dimension": "professional",
            "title_bonus_percent": 10,
            "title_emoji": "🏅",
        }
        body.update(overrides)
        return self.post("/api/epics", body)

    def test_create_and_complete_epic(self) -> None:
        status, payload = self._create_epic()
        self.assertEqual(status, 200)
        epic_id = payload["event"]["id"]
        status, completed = self.post(f"/api/epics/{epic_id}/complete", {"engraving": "完整铭文"})
        self.assertEqual(status, 200)
        self.assertTrue(completed["state"]["titles"][epic_id]["unlocked"])

    def test_complete_epic_rejects_short_engraving(self) -> None:
        _, created = self._create_epic()
        status, payload = self.post(f"/api/epics/{created['event']['id']}/complete", {"engraving": "短"})
        self.assertEqual(status, 400)
        self.assertIn("铭文", payload["error"])

    def test_create_epic_rejects_san_main_dimension(self) -> None:
        status, payload = self._create_epic(main_dimension="san")
        self.assertEqual(status, 400)
        self.assertIn("主维度", payload["error"])

    def test_update_epic(self) -> None:
        _, created = self._create_epic()
        epic_id = created["event"]["id"]
        status, payload = self.post(
            f"/api/epics/{epic_id}/update",
            {
                "title": "更新版",
                "description": "新描述",
                "main_dimension": "knowledge",
                "title_bonus_dimension": "knowledge",
                "title_bonus_percent": 15,
                "title_emoji": "📚",
            },
        )
        self.assertEqual(status, 200)
        self.assertEqual(payload["state"]["epics"][epic_id]["title"], "更新版")


class TitleAndSystemApiTests(HttpTestCase):
    def test_equip_unequip_via_api(self) -> None:
        _, epic = self.post(
            "/api/epics",
            {
                "title": "称号链路",
                "description": "",
                "main_dimension": "professional",
                "title_bonus_dimension": "professional",
                "title_bonus_percent": 5,
            },
        )
        epic_id = epic["event"]["id"]
        self.post(f"/api/epics/{epic_id}/complete", {"engraving": "结项铭文"})
        status, payload = self.post("/api/titles/equip", {"title_id": epic_id})
        self.assertEqual(status, 200)
        self.assertIn(epic_id, payload["state"]["equipped"])
        status, payload = self.post("/api/titles/unequip", {"title_id": epic_id})
        self.assertEqual(status, 200)
        self.assertNotIn(epic_id, payload["state"]["equipped"])

    def test_awaken_once_only(self) -> None:
        status, payload = self.post("/api/awaken", {})
        self.assertEqual(status, 200)
        status, payload = self.post("/api/awaken", {})
        self.assertEqual(status, 400)
        self.assertIn("觉醒", payload["error"])

    def test_system_tick(self) -> None:
        status, payload = self.post("/api/system/tick", {})
        self.assertEqual(status, 200)
        self.assertTrue(payload["ok"])

    def test_delete_event_via_api(self) -> None:
        _, task = self.post(
            "/api/tasks",
            {"title": "删事件", "epic_id": "", "repeatable": False, "effects": [{"dimension": "san", "delta": -1}]},
        )
        _, logged = self.post(f"/api/tasks/{task['event']['id']}/log", {})
        event_id = logged["event"]["event_id"]
        status, payload = self.post(f"/api/events/{event_id}/delete", {"note": ""})
        self.assertEqual(status, 200)
        stream_item = next(e for e in payload["state"]["event_stream"] if e["event_id"] == event_id)
        self.assertTrue(stream_item["deleted"])


class ErrorContractTests(HttpTestCase):
    def test_unknown_api_404(self) -> None:
        status, payload = self.post("/api/no-such-endpoint", {})
        self.assertEqual(status, 404)
        self.assertFalse(payload["ok"])

    def test_invalid_json_body_400(self) -> None:
        request = urllib.request.Request(
            f"{self.base_url}/api/tasks",
            data=b"{ not json",
            headers={"Content-Type": "application/json"},
            method="POST",
        )
        try:
            with urllib.request.urlopen(request, timeout=5) as response:
                status, payload = response.status, json.loads(response.read().decode("utf-8"))
        except urllib.error.HTTPError as exc:
            status, payload = exc.code, json.loads(exc.read().decode("utf-8"))
        self.assertEqual(status, 400)
        self.assertIn("JSON", payload["error"])


class StaticFileTests(HttpTestCase):
    def test_index_served(self) -> None:
        status, body = self.get_raw("/")
        self.assertEqual(status, 200)
        self.assertIn("MicroStep", body)

    def test_js_module_served_with_mime(self) -> None:
        with urllib.request.urlopen(f"{self.base_url}/js/main.js", timeout=5) as response:
            self.assertEqual(response.status, 200)
            self.assertEqual(response.headers["Content-Type"], "application/javascript; charset=utf-8")

    def test_path_traversal_blocked(self) -> None:
        status, _ = self.get_raw("/../backend/server.py")
        self.assertIn(status, (403, 404))


if __name__ == "__main__":
    unittest.main(verbosity=2)
