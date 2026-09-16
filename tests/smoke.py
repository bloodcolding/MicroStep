from __future__ import annotations

import sys
from datetime import date, timedelta
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))

from backend.store import EventStore


def main() -> None:
    workspace = Path(__file__).resolve().parents[1]
    test_path = workspace / "data" / "smoke_events.jsonl"
    test_path.unlink(missing_ok=True)
    store = EventStore(test_path)
    state = store.get_state()
    assert state["profile"]["created_at"] is not None
    assert "epic_infinite_progress" in state["epics"]
    assert "willpower" not in state["dimensions"]
    assert "willpower_gauge" not in state["dimensions"]
    assert "total_exp" not in state["derived"]
    assert "level" not in state["derived"]

    # 手动 Task：多属性变化，手动定义正负值。
    morning = store.create_task(
        "早起",
        None,
        effects=[
            {"dimension": "knowledge", "delta": 5},
            {"dimension": "san", "delta": 2},
        ],
        repeatable=True,
    )
    before_knowledge = store.get_state()["dimensions"]["knowledge"]
    before_san = store.get_state()["dimensions"]["san"]
    assert before_san == 100.0
    store.complete_task(morning["id"])
    state = store.get_state()
    assert state["dimensions"]["knowledge"] == round(before_knowledge + 5, 1)
    assert state["dimensions"]["san"] == min(100.0, round(before_san + 2, 1))
    assert state["tasks"][morning["id"]]["status"] == "active"
    assert state["tasks"][morning["id"]]["times_completed"] == 1

    late_night = store.create_task(
        "熬夜",
        None,
        effects=[
            {"dimension": "professional", "delta": -10},
            {"dimension": "san", "delta": -15},
        ],
        repeatable=False,
    )
    pro_task = store.create_task(
        "专业推进",
        None,
        effects=[{"dimension": "professional", "delta": 50}],
        repeatable=False,
    )
    try:
        store.create_task("不应存在", None, effects=[{"dimension": "willpower", "delta": 5}])
        raise AssertionError("willpower should not exist")
    except ValueError:
        pass
    blocked_task = store.create_task(
        "SAN 不足测试",
        None,
        effects=[{"dimension": "san", "delta": -999}],
        repeatable=False,
    )
    try:
        store.complete_task(blocked_task["id"])
        raise AssertionError("insufficient SAN should block task completion")
    except ValueError:
        pass

    store.complete_task(pro_task["id"])
    before_pro = store.get_state()["dimensions"]["professional"]
    before_san = store.get_state()["dimensions"]["san"]
    late_event = store.complete_task(late_night["id"])
    state = store.get_state()
    assert state["dimensions"]["professional"] == round(max(0, before_pro - 10), 1)
    assert state["dimensions"]["san"] == round(max(0, before_san - 15), 1)
    assert state["tasks"][late_night["id"]]["status"] == "completed"

    # 事件删除只打 deleted 标记，不影响 Task 和状态。
    store.delete_event(late_event["event_id"])
    state = store.get_state()
    assert state["tasks"][late_night["id"]]["status"] == "completed"
    assert state["tasks"][late_night["id"]]["times_completed"] == 1
    deleted_record = next(
        item for item in state["event_stream"] if item["event_id"] == late_event["event_id"]
    )
    assert deleted_record["deleted"] is True
    assert deleted_record["deleted_at"] is not None
    assert deleted_record["created_at"] is not None

    # Task 更新与删除都是独立事件。
    update_event = store.update_task(
        pro_task["id"],
        title="专业推进 v2",
        effects=[
            {"dimension": "professional", "delta": 60},
            {"dimension": "san", "delta": -5},
        ],
        repeatable=True,
    )
    state = store.get_state()
    updated_task = state["tasks"][pro_task["id"]]
    assert updated_task["title"] == "专业推进 v2"
    assert updated_task["repeatable"] is True
    assert updated_task["status"] == "active"
    assert updated_task["effects"] == [
        {"dimension": "professional", "delta": 60.0},
        {"dimension": "san", "delta": -5.0},
    ]
    update_record = next(
        item for item in state["event_stream"] if item["event_id"] == update_event["event_id"]
    )
    assert update_record["created_at"] is not None
    assert update_record["changes"] == [
        {"dimension": "professional", "delta": 60.0},
        {"dimension": "san", "delta": -5.0},
    ]

    delete_event = store.delete_task(late_night["id"])
    state = store.get_state()
    assert state["tasks"][late_night["id"]]["deleted"] is True
    assert state["tasks"][late_night["id"]]["status"] == "deleted"
    delete_record = next(
        item for item in state["event_stream"] if item["event_id"] == delete_event["event_id"]
    )
    assert delete_record["name"].startswith("删除 Task：")
    assert delete_record["created_at"] is not None

    # 里程碑称号：创建时配置属性加成，结项后解锁，装备后作用于所有 Task 变化。
    epic = store.create_epic(
        title="体能里程碑",
        description="完成一项体能挑战",
        main_dimension="physical",
        title_bonus_dimension="physical",
        title_bonus_percent=10,
    )
    store.complete_epic(epic["id"], "持续训练后完成挑战。")
    state = store.get_state()
    assert state["titles"][epic["id"]]["unlocked"] is True
    store.equip_title(epic["id"])
    bonus_task = store.create_task(
        "力量训练",
        epic["id"],
        effects=[{"dimension": "physical", "delta": 100}],
        repeatable=False,
    )
    before_physical = store.get_state()["dimensions"]["physical"]
    bonus_completed = store.complete_task(bonus_task["id"])
    state = store.get_state()
    assert state["dimensions"]["physical"] == round(before_physical + 110, 1)
    assert bonus_completed["effects"][0]["delta"] == 110.0
    assert bonus_completed["effects"][0]["bonus_percent"] == 10.0

    update_epic_event = store.update_epic(
        epic["id"],
        title="体能里程碑 v2",
        description="更新后的里程碑",
        main_dimension="physical",
        title_bonus_dimension="physical",
        title_bonus_percent=20,
        title_emoji="🏋️",
    )
    state = store.get_state()
    assert state["epics"][epic["id"]]["title"] == "体能里程碑 v2"
    assert state["titles"][epic["id"]]["bonus_percent"] == 20.0
    assert state["titles"][epic["id"]]["unlocked"] is True
    second_bonus_task = store.create_task(
        "力量训练二",
        epic["id"],
        effects=[{"dimension": "physical", "delta": 100}],
        repeatable=False,
    )
    before_physical = store.get_state()["dimensions"]["physical"]
    second_completed = store.complete_task(second_bonus_task["id"])
    state = store.get_state()
    assert state["dimensions"]["physical"] == round(before_physical + 120, 1)
    assert second_completed["effects"][0]["delta"] == 120.0
    assert update_epic_event["name"].startswith("更新里程碑：")

    # SAN 每天从 100 开始；跨日时记录上一天最终值。
    current_day = state["daily_san"]["day"]
    final_san = state["daily_san"]["current"]
    next_day = (date.fromisoformat(current_day) + timedelta(days=1)).isoformat()
    store.append_event({"type": "SYSTEM_DAILY_TICK", "name": "每日结算", "changes": [], "date": next_day})
    state = store.get_state()
    assert state["dimensions"]["san"] == 100.0
    assert state["daily_san"]["history"][current_day] == final_san
    assert all(item["created_at"] for item in state["event_stream"])
    assert all(item["name"] for item in state["event_stream"])

    print("smoke ok")
    print(f"knowledge={state['dimensions']['knowledge']}, san={state['dimensions']['san']}")
    test_path.unlink(missing_ok=True)


if __name__ == "__main__":
    main()
