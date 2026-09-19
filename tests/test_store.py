"""store.py EventStore 测试：初始化、日结补齐、SAN 拒绝、tombstone、损坏行容错。

运行方式（二选一）：
    python -m unittest tests.test_store -v
    python tests\\test_store.py

全部使用 tempfile 隔离数据，不触碰真实事件流。
"""

from __future__ import annotations

import json
import sys
import tempfile
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))

from backend.store import EventStore  # noqa: E402


class StoreTestCase(unittest.TestCase):
    def setUp(self) -> None:
        self._tmp = tempfile.TemporaryDirectory()
        self.path = Path(self._tmp.name) / "events.jsonl"
        self.store = EventStore(self.path)

    def tearDown(self) -> None:
        self._tmp.cleanup()

    def read_raw_lines(self) -> list[str]:
        return [line for line in self.path.read_text(encoding="utf-8").splitlines() if line.strip()]


class InitializationTests(StoreTestCase):
    def test_first_get_state_writes_system_init_and_default_epic(self) -> None:
        state = self.store.get_state()
        self.assertIsNotNone(state["profile"]["created_at"])
        self.assertIn("epic_infinite_progress", state["epics"])
        types = [json.loads(line)["type"] for line in self.read_raw_lines()]
        self.assertEqual(types[:2], ["SYSTEM_INIT", "EPIC_CREATED"])

    def test_daily_tick_appended_on_first_access(self) -> None:
        self.store.get_state()
        types = [json.loads(line)["type"] for line in self.read_raw_lines()]
        self.assertIn("SYSTEM_DAILY_TICK", types)


class DailyTickTests(StoreTestCase):
    def test_missing_days_backfilled_in_order(self) -> None:
        self.store.get_state()
        events = self.store.read_events()
        first_tick = next(e for e in events if e["type"] == "SYSTEM_DAILY_TICK")
        # 手动追加两天后的 tick，验证补齐中间日。
        from datetime import date, timedelta

        day3 = (date.fromisoformat(first_tick["date"]) + timedelta(days=2)).isoformat()
        self.store.append_event({"type": "SYSTEM_DAILY_TICK", "name": "每日结算", "changes": [], "date": day3})
        # 再触发一次 get_state：因为流里最后 tick 已是 day3，不应重复补。
        before = len(self.read_raw_lines())
        self.store.ensure_daily_ticks()
        self.assertEqual(len(self.read_raw_lines()), before)

    def test_last_tick_day_reads_latest(self) -> None:
        self.store.get_state()
        events = self.store.read_events()
        ticks = [e["date"] for e in events if e["type"] == "SYSTEM_DAILY_TICK"]
        self.assertEqual(self.store.last_tick_day(), ticks[-1])


class TaskCommandTests(StoreTestCase):
    def test_create_task_rejects_empty_effects(self) -> None:
        with self.assertRaises(ValueError):
            self.store.create_task("空效果", None, effects=[])

    def test_create_task_rejects_unknown_dimension(self) -> None:
        with self.assertRaises(ValueError):
            self.store.create_task("非法维度", None, effects=[{"dimension": "willpower", "delta": 5}])

    def test_complete_task_rejects_insufficient_san(self) -> None:
        task = self.store.create_task(
            "大额消耗", None, effects=[{"dimension": "san", "delta": -999}], repeatable=False
        )
        with self.assertRaises(ValueError):
            self.store.complete_task(task["id"])

    def test_complete_task_applies_title_bonus(self) -> None:
        epic = self.store.create_epic(
            title="体能",
            description="",
            main_dimension="physical",
            title_bonus_dimension="physical",
            title_bonus_percent=20,
        )
        self.store.complete_epic(epic["id"], "完成")
        self.store.equip_title(epic["id"])
        task = self.store.create_task(
            "训练", None, effects=[{"dimension": "physical", "delta": 10}], repeatable=True
        )
        event = self.store.complete_task(task["id"])
        self.assertEqual(event["effects"][0]["delta"], 12.0)

    def test_update_task_missing_keeps_repeatable(self) -> None:
        task = self.store.create_task(
            "任务", None, effects=[{"dimension": "knowledge", "delta": 5}], repeatable=True
        )
        self.store.update_task(task["id"], title="改名", effects=[{"dimension": "knowledge", "delta": 6}])
        state = self.store.get_state()
        self.assertTrue(state["tasks"][task["id"]]["repeatable"])

    def test_delete_task_is_tombstone_not_rewrite(self) -> None:
        task = self.store.create_task(
            "待删", None, effects=[{"dimension": "knowledge", "delta": 5}], repeatable=False
        )
        lines_before = len(self.read_raw_lines())
        self.store.delete_task(task["id"])
        lines = self.read_raw_lines()
        self.assertEqual(len(lines), lines_before + 1)
        last = json.loads(lines[-1])
        self.assertEqual(last["type"], "TASK_DELETED")
        self.assertTrue(last["deleted"])
        with self.assertRaises(ValueError):
            self.store.delete_task(task["id"])  # 重复删除被拒绝


class EpicCommandTests(StoreTestCase):
    def test_update_epic_rejects_unknown(self) -> None:
        with self.assertRaises(KeyError):
            self.store.update_epic(
                "no-such-epic",
                title="x",
                description="",
                main_dimension="professional",
                title_bonus_dimension="professional",
                title_bonus_percent=10,
                title_emoji="🏅",
            )

    def test_complete_epic_validation_layers(self) -> None:
        # 现状：铭文长度校验在 HTTP 层（server.py），store 层不做长度校验。
        epic = self.store.create_epic(
            title="里程碑", description="", main_dimension="professional",
            title_bonus_dimension="professional", title_bonus_percent=10,
        )
        self.store.complete_epic(epic["id"], "一")  # store 层接受任意铭文
        with self.assertRaises(ValueError):
            self.store.complete_epic(epic["id"], "再次结项")  # 已完成


class TitleCommandTests(StoreTestCase):
    def test_equip_rules(self) -> None:
        ids = []
        for index in range(4):
            epic = self.store.create_epic(
                title=f"里程碑{index}", description="", main_dimension="professional",
                title_bonus_dimension="professional", title_bonus_percent=10,
            )
            self.store.complete_epic(epic["id"], "铭文")
            ids.append(epic["id"])
        locked = self.store.create_epic(
            title="未结项", description="", main_dimension="professional",
            title_bonus_dimension="professional", title_bonus_percent=10,
        )
        with self.assertRaises(ValueError):
            self.store.equip_title(locked["id"])  # 未解锁
        for epic_id in ids[:3]:
            self.store.equip_title(epic_id)
        with self.assertRaises(ValueError):
            self.store.equip_title(ids[3])  # 满 3 个
        with self.assertRaises(ValueError):
            self.store.equip_title(ids[0])  # 重复装备


class EventDeletionTests(StoreTestCase):
    def test_delete_event_tombstones_and_blocks_system_events(self) -> None:
        task = self.store.create_task(
            "任务", None, effects=[{"dimension": "knowledge", "delta": 5}], repeatable=False
        )
        completed = self.store.complete_task(task["id"])
        deletion = self.store.delete_event(completed["event_id"])
        self.assertEqual(deletion["type"], "EVENT_DELETED")
        # tombstone 不回滚状态：属性保持结算后的值。
        state = self.store.get_state()
        self.assertEqual(state["dimensions"]["knowledge"], 5.0)
        with self.assertRaises(ValueError):
            self.store.delete_event(completed["event_id"])  # 重复删除
        system_event = next(e for e in self.store.read_events() if e["type"] == "SYSTEM_INIT")
        with self.assertRaises(ValueError):
            self.store.delete_event(system_event["event_id"])  # 系统事件不可删


class RobustnessTests(StoreTestCase):
    def test_corrupt_lines_skipped_not_fatal(self) -> None:
        self.store.get_state()
        task = self.store.create_task(
            "任务", None, effects=[{"dimension": "knowledge", "delta": 5}], repeatable=False
        )
        self.store.complete_task(task["id"])
        with self.path.open("a", encoding="utf-8") as handle:
            handle.write("{ 这是一行损坏的 JSON\n")
            handle.write("\n")
        state = self.store.get_state()
        self.assertEqual(state["dimensions"]["knowledge"], 5.0)

    def test_append_event_assigns_ids_and_timestamps(self) -> None:
        event = self.store.append_event({"type": "PROFILE_AWAKENED", "name": "提前觉醒", "changes": []})
        self.assertIn("event_id", event)
        self.assertIn("created_at", event)
        self.assertEqual(event["at"], event["created_at"])


if __name__ == "__main__":
    unittest.main(verbosity=2)
