from __future__ import annotations

from datetime import date, datetime, timedelta
from typing import Any


# ---------------------------------------------------------------------------
# 属性定义
# ---------------------------------------------------------------------------

DIMENSION_META: dict[str, dict[str, Any]] = {
    "san": {
        "name": "🧠 SAN",
        "full_name": "理智 / 精力槽",
        "kind": "gauge",
        "description": "每天从 100 开始，按当天事件结算，次日记录最终值后重置。",
        "tags": ["恢复"],
    },
    "physical": {
        "name": "🔋 体质",
        "full_name": "硬件基底",
        "kind": "pool",
        "description": "健康与体能。",
        "tags": ["健康", "体能", "运动"],
    },
    "professional": {
        "name": "⚔️ 专业能力",
        "full_name": "核心资产",
        "kind": "pool",
        "description": "职业、项目与核心能力。",
        "tags": ["工作", "项目", "专业"],
    },
    "knowledge": {
        "name": "📚 知识",
        "full_name": "认知广度",
        "kind": "pool",
        "description": "阅读、学习与认知积累。",
        "tags": ["阅读", "学习", "知识"],
    },
    "expression": {
        "name": "🗣️ 表达",
        "full_name": "输出信噪比",
        "kind": "pool",
        "description": "写作、演讲、交流与语言输出。",
        "tags": ["写作", "表达", "输出"],
    },
    "kindness": {
        "name": "🕊️ 良善",
        "full_name": "正外部性",
        "kind": "pool",
        "description": "助人、贡献与社会责任。",
        "tags": ["助人", "公益", "贡献"],
    },
    "charm": {
        "name": "🌟 魅力",
        "full_name": "外部杠杆",
        "kind": "pool",
        "description": "形象、社交与影响力。",
        "tags": ["社交", "形象", "影响力"],
    },
}

POOL_DIMENSIONS = [key for key, meta in DIMENSION_META.items() if meta["kind"] == "pool"]
EFFECT_DIMENSIONS = ["san", *POOL_DIMENSIONS]
EFFECT_DIMENSION_SET = set(EFFECT_DIMENSIONS)
TASK_EFFECT_DIMENSIONS = list(EFFECT_DIMENSIONS)
TASK_EFFECT_DIMENSION_SET = set(TASK_EFFECT_DIMENSIONS)
TITLE_BONUS_DIMENSIONS = list(POOL_DIMENSIONS)


def iso_date(value: date | datetime | str | None = None) -> str:
    if value is None:
        return date.today().isoformat()
    if isinstance(value, datetime):
        return value.date().isoformat()
    if isinstance(value, date):
        return value.isoformat()
    return str(value)


def now_iso() -> str:
    return datetime.now().astimezone().isoformat(timespec="seconds")


def clamp(value: float, low: float = 0.0, high: float = 100.0) -> float:
    return max(low, min(high, value))


def _parse_day(value: Any) -> date:
    if isinstance(value, date):
        return value
    return date.fromisoformat(str(value))


# ---------------------------------------------------------------------------
# 初始状态与 Reducer
# ---------------------------------------------------------------------------


def initial_state() -> dict[str, Any]:
    return {
        "profile": {
            "name": "成长者",
            "created_at": None,
            "awakened": False,
            "action_count": 0,
        },
        "dimensions": {
            "san": 100.0,
            **{key: 0.0 for key in POOL_DIMENSIONS},
        },
        "meta": {
            "current_day": iso_date(),
            "last_tick_day": None,
            "daily_san_history": {},
        },
        "epics": {},
        "tasks": {},
        "titles": {},
        "equipped": [],
        "logs": [],
    }


def apply_event(state: dict[str, Any], event: dict[str, Any]) -> dict[str, Any]:
    event_type = event.get("type")
    if event_type == "SYSTEM_INIT":
        state["profile"]["created_at"] = event.get("date") or iso_date()
        state["meta"]["current_day"] = state["profile"]["created_at"]
        return state
    if event_type == "SYSTEM_DAILY_TICK":
        return _apply_daily_tick(state, event)
    if event_type == "PROFILE_AWAKENED":
        state["profile"]["awakened"] = True
        return state
    if event_type == "TASK_CREATED":
        _apply_task_created(state, event)
        return state
    if event_type == "TASK_COMPLETED":
        _apply_task_completed(state, event)
        return state
    if event_type == "TASK_UPDATED":
        _apply_task_updated(state, event)
        return state
    if event_type == "TASK_DELETED":
        _apply_task_deleted(state, event)
        return state
    if event_type == "EPIC_CREATED":
        _apply_epic_created(state, event)
        return state
    if event_type == "EPIC_UPDATED":
        _apply_epic_updated(state, event)
        return state
    if event_type == "EPIC_COMPLETED":
        _apply_epic_completed(state, event)
        return state
    if event_type == "TITLE_EQUIPPED":
        _apply_title_equipped(state, event)
        return state
    if event_type == "TITLE_UNEQUIPPED":
        _apply_title_unequipped(state, event)
        return state
    # 旧 ACTION_LOGGED 只作为历史事件保留，不再参与状态结算。
    return state


def _apply_daily_tick(state: dict[str, Any], event: dict[str, Any]) -> dict[str, Any]:
    new_day = event.get("date") or iso_date()
    previous_day = state["meta"].get("current_day")
    if previous_day and previous_day != new_day:
        history = state["meta"].setdefault("daily_san_history", {})
        history[previous_day] = round(clamp(float(state["dimensions"]["san"]), 0, 100), 1)
    state["meta"]["current_day"] = new_day
    state["meta"]["last_tick_day"] = new_day
    state["dimensions"]["san"] = 100.0
    return state


# ---------------------------------------------------------------------------
# Task
# ---------------------------------------------------------------------------


def _apply_task_created(state: dict[str, Any], event: dict[str, Any]) -> None:
    task_id = event["id"]
    effects = _effects_from_event(event)
    state["tasks"][task_id] = {
        "id": task_id,
        "event_id": event.get("event_id"),
        "title": event.get("title", "未命名 Task"),
        "epic_id": event.get("epic_id"),
        "effects": effects,
        "status": "active",
        "deleted": False,
        "deleted_at": None,
        "updated_at": None,
        "created_at": event.get("date") or iso_date(),
        "completed_at": None,
        "progress": 0,
        "repeatable": bool(event.get("repeatable")),
        "times_completed": 0,
        "last_completed_at": None,
        "tags": event.get("tags") or [],
    }
    epic_id = event.get("epic_id")
    if epic_id and epic_id in state["epics"]:
        task_ids = state["epics"][epic_id].setdefault("task_ids", [])
        if task_id not in task_ids:
            task_ids.append(task_id)


def _apply_task_completed(state: dict[str, Any], event: dict[str, Any]) -> None:
    task_id = event["id"]
    task = state["tasks"].get(task_id)
    if not task or task.get("deleted"):
        return
    action_day = event.get("date") or iso_date()
    effects = _effects_from_event(event) or _effects_from_event(task)
    for effect in effects:
        dimension = effect["dimension"]
        delta = float(effect["delta"])
        if dimension == "san":
            state["dimensions"]["san"] = clamp(float(state["dimensions"]["san"]) + delta, 0, 100)
        elif dimension in state["dimensions"]:
            current = float(state["dimensions"].get(dimension, 0.0))
            state["dimensions"][dimension] = round(max(0.0, current + delta), 1)

    task["times_completed"] = int(task.get("times_completed", 0)) + 1
    task["last_completed_at"] = action_day
    task["progress"] = 100 if not task.get("repeatable") else min(100, task["times_completed"] * 10)
    if task.get("repeatable"):
        task["status"] = "active"
    else:
        task["status"] = "completed"
        task["completed_at"] = action_day
    state["logs"].append(
        {
            "type": "task",
            "id": task_id,
            "event_id": event.get("event_id"),
            "title": task.get("title"),
            "effects": effects,
            "repeatable": bool(task.get("repeatable")),
            "times_completed": task["times_completed"],
            "date": action_day,
            "at": event.get("at"),
        }
    )


def _apply_task_updated(state: dict[str, Any], event: dict[str, Any]) -> None:
    task_id = event["id"]
    task = state["tasks"].get(task_id)
    if not task or task.get("deleted"):
        return
    old_epic_id = task.get("epic_id")
    if event.get("title"):
        task["title"] = event["title"]
    if "epic_id" in event:
        task["epic_id"] = event.get("epic_id")
    if "repeatable" in event:
        was_completed = task.get("status") == "completed"
        task["repeatable"] = bool(event.get("repeatable"))
        if task["repeatable"] and was_completed:
            task["status"] = "active"
            task["completed_at"] = None
            task["progress"] = min(100, int(task.get("times_completed", 0)) * 10)
    effects = _effects_from_event(event)
    if effects:
        task["effects"] = effects
    task["updated_at"] = event.get("created_at") or event.get("at") or event.get("date") or iso_date()
    _sync_task_epic(state, task_id, old_epic_id, task.get("epic_id"))


def _apply_task_deleted(state: dict[str, Any], event: dict[str, Any]) -> None:
    task_id = event["id"]
    task = state["tasks"].get(task_id)
    if not task or task.get("deleted"):
        return
    task["deleted"] = True
    task["deleted_at"] = event.get("deleted_at") or event.get("created_at") or event.get("at") or iso_date()
    task["status"] = "deleted"
    epic_id = task.get("epic_id")
    if epic_id and epic_id in state["epics"]:
        task_ids = state["epics"][epic_id].get("task_ids", [])
        if task_id in task_ids:
            task_ids.remove(task_id)


def _sync_task_epic(
    state: dict[str, Any],
    task_id: str,
    old_epic_id: str | None,
    new_epic_id: str | None,
) -> None:
    if old_epic_id and old_epic_id in state["epics"] and old_epic_id != new_epic_id:
        task_ids = state["epics"][old_epic_id].get("task_ids", [])
        if task_id in task_ids:
            task_ids.remove(task_id)
    if new_epic_id and new_epic_id in state["epics"]:
        task_ids = state["epics"][new_epic_id].setdefault("task_ids", [])
        if task_id not in task_ids:
            task_ids.append(task_id)


# ---------------------------------------------------------------------------
# 里程碑与称号
# ---------------------------------------------------------------------------


def _apply_epic_created(state: dict[str, Any], event: dict[str, Any]) -> None:
    epic_id = event["id"]
    title = event.get("title", "未命名里程碑")
    bonus_dimension = event.get("title_bonus_dimension") or "professional"
    if bonus_dimension not in TITLE_BONUS_DIMENSIONS:
        bonus_dimension = "professional"
    try:
        bonus_percent = round(float(event.get("title_bonus_percent") or 0), 2)
    except (TypeError, ValueError):
        bonus_percent = 0.0
    state["epics"][epic_id] = {
        "id": epic_id,
        "title": title,
        "description": event.get("description", ""),
        "main_dimension": event.get("main_dimension", "professional"),
        "title_id": epic_id,
        "title_emoji": event.get("title_emoji") or "🏅",
        "title_bonus_dimension": bonus_dimension,
        "title_bonus_percent": bonus_percent,
        "status": "active",
        "task_ids": [],
        "created_at": event.get("date") or iso_date(),
        "completed_at": None,
        "engraving": "",
    }
    state["titles"][epic_id] = {
        "id": epic_id,
        "name": title,
        "emoji": event.get("title_emoji") or "🏅",
        "description": _title_description(title, bonus_dimension, bonus_percent),
        "milestone_id": epic_id,
        "target_dimension": bonus_dimension,
        "bonus_percent": bonus_percent,
        "unlocked": False,
    }


def _apply_epic_updated(state: dict[str, Any], event: dict[str, Any]) -> None:
    epic_id = event["id"]
    epic = state["epics"].get(epic_id)
    if not epic:
        return
    title = event.get("title") or epic.get("title", "未命名里程碑")
    main_dimension = event.get("main_dimension") or epic.get("main_dimension", "professional")
    bonus_dimension = event.get("title_bonus_dimension") or epic.get("title_bonus_dimension", "professional")
    if bonus_dimension not in TITLE_BONUS_DIMENSIONS:
        bonus_dimension = "professional"
    try:
        bonus_percent = round(float(event.get("title_bonus_percent", epic.get("title_bonus_percent", 0))), 2)
    except (TypeError, ValueError):
        bonus_percent = 0.0
    epic["title"] = title
    epic["description"] = event.get("description", epic.get("description", ""))
    epic["main_dimension"] = main_dimension
    epic["title_emoji"] = event.get("title_emoji") or epic.get("title_emoji") or "🏅"
    epic["title_bonus_dimension"] = bonus_dimension
    epic["title_bonus_percent"] = bonus_percent
    epic_title = state["titles"].get(epic_id)
    if epic_title:
        epic_title["name"] = title
        epic_title["emoji"] = epic["title_emoji"]
        epic_title["description"] = _title_description(title, bonus_dimension, bonus_percent)
        epic_title["target_dimension"] = bonus_dimension
        epic_title["bonus_percent"] = bonus_percent


def _title_description(title: str, dimension: str, bonus_percent: float) -> str:
    dimension_name = DIMENSION_META.get(dimension, DIMENSION_META["professional"])["name"]
    return f"完成里程碑「{title}」解锁；{dimension_name}变化 {bonus_percent:g}%"


def _apply_epic_completed(state: dict[str, Any], event: dict[str, Any]) -> None:
    epic_id = event["id"]
    epic = state["epics"].get(epic_id)
    if not epic:
        return
    epic["status"] = "completed"
    epic["completed_at"] = event.get("date") or iso_date()
    epic["engraving"] = event.get("engraving", "")
    title = state["titles"].get(epic_id)
    if title:
        title["unlocked"] = True
    state["logs"].append(
        {
            "type": "epic",
            "id": epic_id,
            "event_id": event.get("event_id"),
            "title": epic.get("title"),
            "engraving": epic.get("engraving"),
            "date": event.get("date") or iso_date(),
            "at": event.get("at"),
        }
    )


def _apply_title_equipped(state: dict[str, Any], event: dict[str, Any]) -> None:
    title_id = event.get("title_id")
    title = state["titles"].get(title_id)
    if not title or not title.get("unlocked") or title_id in state["equipped"]:
        return
    if len(state["equipped"]) < 3:
        state["equipped"].append(title_id)


def _apply_title_unequipped(state: dict[str, Any], event: dict[str, Any]) -> None:
    title_id = event.get("title_id")
    if title_id in state["equipped"]:
        state["equipped"].remove(title_id)


def apply_title_bonuses(effects: list[dict[str, Any]], state: dict[str, Any]) -> list[dict[str, Any]]:
    """把已装备称号的百分比加成应用到 Task 的每项属性变化。"""
    bonuses: dict[str, float] = {}
    for title_id in state.get("equipped", []):
        title = state["titles"].get(title_id)
        if not title or not title.get("unlocked"):
            continue
        dimension = title.get("target_dimension")
        if dimension not in TITLE_BONUS_DIMENSIONS:
            continue
        bonuses[dimension] = bonuses.get(dimension, 0.0) + float(title.get("bonus_percent") or 0)

    result: list[dict[str, Any]] = []
    for effect in effects:
        dimension = effect.get("dimension")
        if dimension not in EFFECT_DIMENSION_SET:
            continue
        try:
            base_delta = round(float(effect.get("delta", 0)), 1)
        except (TypeError, ValueError):
            continue
        if base_delta == 0:
            continue
        bonus_percent = bonuses.get(dimension, 0.0)
        final_delta = round(base_delta * (1.0 + bonus_percent / 100.0), 1)
        result.append(
            {
                "dimension": dimension,
                "delta": final_delta,
                "base_delta": base_delta,
                "bonus_percent": bonus_percent,
            }
        )
    return result


# ---------------------------------------------------------------------------
# 事件规范化
# ---------------------------------------------------------------------------


def _effects_from_event(event: dict[str, Any]) -> list[dict[str, Any]]:
    raw_effects = event.get("effects")
    effects: list[dict[str, Any]] = []
    if isinstance(raw_effects, list):
        for item in raw_effects:
            if not isinstance(item, dict):
                continue
            dimension = item.get("dimension")
            if dimension not in EFFECT_DIMENSION_SET:
                continue
            try:
                delta = round(float(item.get("delta", 0)), 1)
            except (TypeError, ValueError):
                continue
            if delta != 0:
                effects.append({"dimension": dimension, "delta": delta})
    if effects:
        return effects

    # 兼容旧版单属性 Task 事件。
    dimension = event.get("dimension")
    if dimension not in EFFECT_DIMENSION_SET:
        return []
    try:
        exp_delta = round(float(event.get("exp_delta", event.get("base_exp", 0))), 1)
    except (TypeError, ValueError):
        exp_delta = 0.0
    try:
        san_delta = round(float(event.get("san_delta", 0)), 1)
    except (TypeError, ValueError):
        san_delta = 0.0
    if dimension != "san" and exp_delta != 0:
        effects.append({"dimension": dimension, "delta": exp_delta})
    if san_delta != 0:
        effects.append({"dimension": "san", "delta": san_delta})
    return effects


# ---------------------------------------------------------------------------
# 状态与事件流
# ---------------------------------------------------------------------------


def build_state(events: list[dict[str, Any]]) -> dict[str, Any]:
    state = initial_state()
    for event in events:
        if event.get("type") in ("EVENT_DELETED", "STREAM_CLEARED"):
            continue
        state = apply_event(state, event)

    state["derived"] = {
        "san": round(float(state["dimensions"]["san"]), 1),
        "san_start": 100.0,
        "san_change": round(float(state["dimensions"]["san"]) - 100.0, 1),
        "total_actions": int(state["profile"].get("action_count", 0)),
        "pool_dimensions": POOL_DIMENSIONS,
    }
    state["daily_san"] = {
        "day": state["meta"].get("current_day", iso_date()),
        "start": 100.0,
        "current": round(float(state["dimensions"]["san"]), 1),
        "change": round(float(state["dimensions"]["san"]) - 100.0, 1),
        "history": dict(state["meta"].get("daily_san_history", {})),
    }
    state["event_stream"] = build_event_stream(events)
    state["dimensions"] = {
        key: (round(value, 1) if isinstance(value, float) else value)
        for key, value in state["dimensions"].items()
    }
    state["titles"] = dict(state["titles"])
    state["equipped"] = list(state["equipped"])
    return state


def build_event_stream(events: list[dict[str, Any]]) -> list[dict[str, Any]]:
    deletions: dict[str, dict[str, Any]] = {}
    for event in events:
        if event.get("type") != "EVENT_DELETED":
            continue
        target = event.get("target_event_id")
        if not target:
            continue
        deletions[target] = {
            "deleted_at": event.get("deleted_at")
            or event.get("created_at")
            or event.get("at")
            or event.get("date"),
            "deletion_event_id": event.get("event_id"),
        }

    stream: list[dict[str, Any]] = []
    for event in events:
        event_type = event.get("type")
        if event_type in ("EVENT_DELETED", "STREAM_CLEARED"):
            continue
        event_id = event.get("event_id") or event.get("id")
        deletion = deletions.get(event_id, {})
        stream.append(
            {
                "event_id": event_id,
                "type": event_type,
                "name": _event_name(event),
                "changes": _event_changes(event),
                "created_at": event.get("created_at") or event.get("at") or event.get("date"),
                "deleted": bool(deletion),
                "deleted_at": deletion.get("deleted_at"),
                "deletion_event_id": deletion.get("deletion_event_id"),
                "details": {
                    "text": event.get("text"),
                    "title": event.get("title"),
                    "tags": event.get("tags") or [],
                    "dimension": event.get("dimension"),
                    "note": event.get("note"),
                    "engraving": event.get("engraving"),
                    "repeatable": bool(event.get("repeatable")),
                },
            }
        )
    return stream


def _event_name(event: dict[str, Any]) -> str:
    explicit = event.get("name")
    if explicit:
        return str(explicit)
    event_type = event.get("type")
    title = event.get("title") or event.get("text") or ""
    mapping = {
        "TASK_CREATED": f"创建 Task：{title}",
        "TASK_COMPLETED": f"记录 Task：{title}",
        "TASK_UPDATED": f"更新 Task：{title}",
        "TASK_DELETED": f"删除 Task：{title}",
        "EPIC_CREATED": f"创建里程碑：{title}",
        "EPIC_UPDATED": f"更新里程碑：{title}",
        "EPIC_COMPLETED": f"里程碑结项：{title}",
        "ACTION_LOGGED": title or "历史记录",
        "PROFILE_AWAKENED": "提前觉醒",
        "TITLE_EQUIPPED": f"装备称号：{event.get('title_id') or ''}",
        "TITLE_UNEQUIPPED": f"卸下称号：{event.get('title_id') or ''}",
        "SYSTEM_DAILY_TICK": "每日结算",
        "SYSTEM_INIT": "系统初始化",
    }
    return str(mapping.get(event_type, event_type or "未知事件"))


def _event_changes(event: dict[str, Any]) -> list[dict[str, Any]]:
    changes = event.get("changes")
    if isinstance(changes, list):
        normalized = _normalize_change_list(changes)
        if normalized:
            return normalized
    return _effects_from_event(event)


def _normalize_change_list(items: list[Any]) -> list[dict[str, Any]]:
    result: list[dict[str, Any]] = []
    for item in items:
        if not isinstance(item, dict):
            continue
        dimension = item.get("dimension")
        if dimension not in EFFECT_DIMENSION_SET:
            continue
        try:
            delta = round(float(item.get("delta", 0)), 1)
        except (TypeError, ValueError):
            continue
        if delta != 0:
            result.append({"dimension": dimension, "delta": delta})
    return result
