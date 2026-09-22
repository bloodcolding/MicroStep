"""domain.py 纯函数单元测试：Reducer 边界、日结、称号加成、旧事件兼容。

运行方式（二选一）：
    python -m unittest tests.test_domain -v
    python tests\\test_domain.py
"""

from __future__ import annotations

import json
import sys
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))

from backend.domain import (  # noqa: E402
    DIMENSION_META,
    POOL_DIMENSIONS,
    apply_event,
    apply_title_bonuses,
    build_state,
    initial_state,
)


def make_task_created(task_id: str = "t1", **overrides) -> dict:
    event = {
        "type": "TASK_CREATED",
        "id": task_id,
        "name": f"创建 Task：{task_id}",
        "changes": [],
        "title": task_id,
        "epic_id": None,
        "effects": [{"dimension": "knowledge", "delta": 5.0}],
        "repeatable": False,
        "date": "2026-09-19",
        "at": "2026-09-19T10:00:00",
    }
    event.update(overrides)
    return event


def make_task_completed(task_id: str = "t1", effects=None, **overrides) -> dict:
    event = {
        "type": "TASK_COMPLETED",
        "id": task_id,
        "name": f"记录 Task：{task_id}",
        "changes": effects or [{"dimension": "knowledge", "delta": 5.0}],
        "effects": effects or [{"dimension": "knowledge", "delta": 5.0}],
        "note": "",
        "date": "2026-09-19",
        "at": "2026-09-19T10:05:00",
    }
    event.update(overrides)
    return event


def make_epic_created(epic_id: str = "e1", **overrides) -> dict:
    event = {
        "type": "EPIC_CREATED",
        "id": epic_id,
        "name": f"创建里程碑：{epic_id}",
        "changes": [],
        "title": epic_id,
        "description": "",
        "main_dimension": "professional",
        "title_emoji": "🏅",
        "title_bonus_dimension": "professional",
        "title_bonus_percent": 10,
        "date": "2026-09-19",
        "at": "2026-09-19T10:00:00",
    }
    event.update(overrides)
    return event


class InitialStateTests(unittest.TestCase):
    def test_dimensions_shape(self) -> None:
        state = initial_state()
        self.assertEqual(set(state["dimensions"]), {"san", *POOL_DIMENSIONS})
        self.assertEqual(state["dimensions"]["san"], 100.0)
        for key in POOL_DIMENSIONS:
            self.assertEqual(state["dimensions"][key], 0.0)

    def test_no_willpower_or_level_system(self) -> None:
        state = initial_state()
        self.assertNotIn("willpower", state["dimensions"])
        self.assertNotIn("total_exp", state["derived"] if "derived" in state else {})
        self.assertNotIn("level", state["derived"] if "derived" in state else {})
        self.assertNotIn("willpower", DIMENSION_META)


class TaskReducerTests(unittest.TestCase):
    def test_task_created_registers_task_and_epic_link(self) -> None:
        state = initial_state()
        apply_event(state, make_epic_created("e1"))
        apply_event(state, make_task_created("t1", epic_id="e1"))
        task = state["tasks"]["t1"]
        self.assertEqual(task["status"], "active")
        self.assertEqual(task["effects"], [{"dimension": "knowledge", "delta": 5.0}])
        self.assertIn("t1", state["epics"]["e1"]["task_ids"])

    def test_task_completed_settles_effects(self) -> None:
        state = initial_state()
        apply_event(state, make_task_created("t1"))
        apply_event(state, make_task_completed("t1"))
        self.assertEqual(state["dimensions"]["knowledge"], 5.0)
        self.assertEqual(state["tasks"]["t1"]["status"], "completed")
        self.assertEqual(len(state["logs"]), 1)

    def test_pool_clamps_at_zero(self) -> None:
        state = initial_state()
        apply_event(state, make_task_created("t1"))
        apply_event(state, make_task_completed("t1", effects=[{"dimension": "physical", "delta": -50.0}]))
        self.assertEqual(state["dimensions"]["physical"], 0.0)

    def test_san_clamps_at_100(self) -> None:
        state = initial_state()
        apply_event(state, make_task_created("t1"))
        apply_event(state, make_task_completed("t1", effects=[{"dimension": "san", "delta": 30.0}]))
        self.assertEqual(state["dimensions"]["san"], 100.0)

    def test_repeatable_task_stays_active(self) -> None:
        state = initial_state()
        apply_event(state, make_task_created("t1", repeatable=True))
        apply_event(state, make_task_completed("t1"))
        apply_event(state, make_task_completed("t1"))
        task = state["tasks"]["t1"]
        self.assertEqual(task["status"], "active")
        self.assertEqual(task["times_completed"], 2)
        self.assertEqual(state["dimensions"]["knowledge"], 10.0)

    def test_completed_event_for_unknown_task_ignored(self) -> None:
        state = initial_state()
        apply_event(state, make_task_completed("ghost"))
        self.assertNotIn("ghost", state["tasks"])
        self.assertEqual(state["dimensions"]["knowledge"], 0.0)
        self.assertEqual(state["logs"], [])

    def test_completed_event_for_deleted_task_ignored(self) -> None:
        state = initial_state()
        apply_event(state, make_task_created("t1"))
        apply_event(
            state,
            {"type": "TASK_DELETED", "id": "t1", "deleted": True, "deleted_at": "x", "date": "2026-09-19"},
        )
        apply_event(state, make_task_completed("t1"))
        self.assertEqual(state["dimensions"]["knowledge"], 0.0)

    def test_task_updated_revives_completed_once_repeatable(self) -> None:
        state = initial_state()
        apply_event(state, make_task_created("t1"))
        apply_event(state, make_task_completed("t1"))
        apply_event(
            state,
            {
                "type": "TASK_UPDATED",
                "id": "t1",
                "title": "t1v2",
                "changes": [],
                "effects": [{"dimension": "knowledge", "delta": 8.0}],
                "repeatable": True,
                "date": "2026-09-19",
            },
        )
        task = state["tasks"]["t1"]
        self.assertEqual(task["title"], "t1v2")
        self.assertEqual(task["status"], "active")
        self.assertIsNone(task["completed_at"])
        self.assertEqual(task["effects"], [{"dimension": "knowledge", "delta": 8.0}])

    def test_task_deleted_removes_from_epic(self) -> None:
        state = initial_state()
        apply_event(state, make_epic_created("e1"))
        apply_event(state, make_task_created("t1", epic_id="e1"))
        apply_event(
            state,
            {"type": "TASK_DELETED", "id": "t1", "deleted": True, "deleted_at": "x", "date": "2026-09-19"},
        )
        task = state["tasks"]["t1"]
        self.assertTrue(task["deleted"])
        self.assertEqual(task["status"], "deleted")
        self.assertNotIn("t1", state["epics"]["e1"]["task_ids"])


class DailyTickTests(unittest.TestCase):
    def test_tick_records_history_and_resets_san(self) -> None:
        state = initial_state()
        # ERR-001：initial_state 的 current_day 锚定真实时钟，显式钉死为固定日期使断言确定性。
        state["meta"]["current_day"] = "2026-09-19"
        apply_event(state, make_task_created("t1"))
        apply_event(state, make_task_completed("t1", effects=[{"dimension": "san", "delta": -30.0}]))
        self.assertEqual(state["dimensions"]["san"], 70.0)
        apply_event(state, {"type": "SYSTEM_DAILY_TICK", "date": "2026-09-20", "changes": []})
        self.assertEqual(state["dimensions"]["san"], 100.0)
        self.assertEqual(state["meta"]["daily_san_history"]["2026-09-19"], 70.0)
        self.assertEqual(state["meta"]["last_tick_day"], "2026-09-20")


class EpicAndTitleTests(unittest.TestCase):
    def test_epic_created_builds_paired_title(self) -> None:
        state = initial_state()
        apply_event(state, make_epic_created("e1", title_bonus_percent=15))
        title = state["titles"]["e1"]
        self.assertFalse(title["unlocked"])
        self.assertEqual(title["bonus_percent"], 15.0)
        self.assertEqual(title["target_dimension"], "professional")

    def test_invalid_bonus_dimension_falls_back(self) -> None:
        state = initial_state()
        apply_event(state, make_epic_created("e1", title_bonus_dimension="san"))
        self.assertEqual(state["titles"]["e1"]["target_dimension"], "professional")
        apply_event(state, make_epic_created("e2", title_bonus_percent="not-a-number"))
        self.assertEqual(state["titles"]["e2"]["bonus_percent"], 0.0)

    def test_epic_completed_unlocks_title(self) -> None:
        state = initial_state()
        apply_event(state, make_epic_created("e1"))
        apply_event(
            state,
            {"type": "EPIC_COMPLETED", "id": "e1", "engraving": "刻下印记", "date": "2026-09-19", "changes": []},
        )
        self.assertEqual(state["epics"]["e1"]["status"], "completed")
        self.assertTrue(state["titles"]["e1"]["unlocked"])
        self.assertEqual(state["epics"]["e1"]["engraving"], "刻下印记")

    def test_equip_max_three_enforced_in_reducer(self) -> None:
        state = initial_state()
        for index in range(4):
            apply_event(state, make_epic_created(f"e{index}"))
            apply_event(
                state,
                {"type": "EPIC_COMPLETED", "id": f"e{index}", "engraving": "x", "date": "2026-09-19", "changes": []},
            )
        for index in range(4):
            apply_event(state, {"type": "TITLE_EQUIPPED", "title_id": f"e{index}", "changes": []})
        self.assertEqual(len(state["equipped"]), 3)
        self.assertNotIn("e3", state["equipped"])

    def test_unequip_removes(self) -> None:
        state = initial_state()
        apply_event(state, make_epic_created("e1"))
        apply_event(
            state,
            {"type": "EPIC_COMPLETED", "id": "e1", "engraving": "x", "date": "2026-09-19", "changes": []},
        )
        apply_event(state, {"type": "TITLE_EQUIPPED", "title_id": "e1", "changes": []})
        apply_event(state, {"type": "TITLE_UNEQUIPPED", "title_id": "e1", "changes": []})
        self.assertEqual(state["equipped"], [])

    def test_locked_title_cannot_equip(self) -> None:
        state = initial_state()
        apply_event(state, make_epic_created("e1"))
        apply_event(state, {"type": "TITLE_EQUIPPED", "title_id": "e1", "changes": []})
        self.assertEqual(state["equipped"], [])


class TitleBonusTests(unittest.TestCase):
    def _state_with_titles(self) -> dict:
        state = initial_state()
        for index, percent in enumerate((10, 25)):
            apply_event(state, make_epic_created(f"e{index}", title_bonus_percent=percent))
            apply_event(
                state,
                {"type": "EPIC_COMPLETED", "id": f"e{index}", "engraving": "x", "date": "2026-09-19", "changes": []},
            )
            apply_event(state, {"type": "TITLE_EQUIPPED", "title_id": f"e{index}", "changes": []})
        return state

    def test_bonuses_stack_additively(self) -> None:
        state = self._state_with_titles()
        effects = apply_title_bonuses([{"dimension": "professional", "delta": 100.0}], state)
        self.assertEqual(effects[0]["delta"], 135.0)
        self.assertEqual(effects[0]["bonus_percent"], 35.0)

    def test_zero_delta_dropped_and_unknown_dimension_ignored(self) -> None:
        state = self._state_with_titles()
        effects = apply_title_bonuses(
            [
                {"dimension": "professional", "delta": 0},
                {"dimension": "willpower", "delta": 5},
                {"dimension": "san", "delta": 5},
            ],
            state,
        )
        self.assertEqual([e["dimension"] for e in effects], ["san"])

    def test_negative_delta_amplified_by_bonus(self) -> None:
        state = self._state_with_titles()
        effects = apply_title_bonuses([{"dimension": "professional", "delta": -10.0}], state)
        self.assertEqual(effects[0]["delta"], -13.5)


class LegacyEventTests(unittest.TestCase):
    def test_legacy_single_dimension_event(self) -> None:
        state = initial_state()
        apply_event(
            state,
            {
                "type": "TASK_CREATED",
                "id": "t1",
                "title": "旧版",
                "date": "2026-09-19",
                "dimension": "knowledge",
                "exp_delta": 7,
                "san_delta": -3,
            },
        )
        task = state["tasks"]["t1"]
        self.assertEqual(task["effects"], [{"dimension": "knowledge", "delta": 7.0}, {"dimension": "san", "delta": -3.0}])

    def test_unknown_event_type_ignored(self) -> None:
        state = initial_state()
        before = json.dumps(state, sort_keys=True, default=str)
        apply_event(state, {"type": "SOMETHING_NEW", "date": "2026-09-19"})
        after = json.dumps(state, sort_keys=True, default=str)
        self.assertEqual(before, after)


class BuildStateTests(unittest.TestCase):
    def test_event_deleted_and_stream_cleared_skipped_in_replay(self) -> None:
        events = [
            {"type": "SYSTEM_INIT", "date": "2026-09-19", "changes": []},
            make_task_created("t1"),
            {"type": "EVENT_DELETED", "target_event_id": "x", "changes": []},
            {"type": "STREAM_CLEARED", "changes": []},
        ]
        state = build_state(events)
        self.assertIn("t1", state["tasks"])
        types = {item["type"] for item in state["event_stream"]}
        self.assertNotIn("EVENT_DELETED", types)
        self.assertNotIn("STREAM_CLEARED", types)

    def test_event_stream_marks_tombstoned_event(self) -> None:
        events = [
            {"type": "SYSTEM_INIT", "date": "2026-09-19", "changes": [], "event_id": "ev1"},
            make_task_created("t1") | {"event_id": "ev2"},
            {"type": "EVENT_DELETED", "target_event_id": "ev2", "deleted_at": "2026-09-19T11:00:00", "changes": []},
        ]
        state = build_state(events)
        target = next(item for item in state["event_stream"] if item["event_id"] == "ev2")
        self.assertTrue(target["deleted"])
        self.assertEqual(target["deleted_at"], "2026-09-19T11:00:00")

    def test_derived_san_fields(self) -> None:
        events = [
            {"type": "SYSTEM_INIT", "date": "2026-09-19", "changes": []},
            make_task_created("t1"),
            make_task_completed("t1", effects=[{"dimension": "san", "delta": -20.0}]),
        ]
        state = build_state(events)
        self.assertEqual(state["derived"]["san"], 80.0)
        self.assertEqual(state["derived"]["san_change"], -20.0)
        self.assertEqual(state["daily_san"]["current"], 80.0)


if __name__ == "__main__":
    unittest.main(verbosity=2)
