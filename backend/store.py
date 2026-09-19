from __future__ import annotations

import json
import threading
import uuid
from datetime import date, datetime, timedelta
from pathlib import Path
from typing import Any

from .domain import (
    MAX_EQUIPPED_TITLES,
    TASK_EFFECT_DIMENSION_SET,
    TITLE_BONUS_DIMENSIONS,
    apply_title_bonuses,
    build_state,
    iso_date,
    now_iso,
)


def _normalize_effects(effects: list[dict[str, Any]] | None) -> list[dict[str, Any]]:
    if not isinstance(effects, list):
        return []
    merged: dict[str, float] = {}
    for item in effects:
        if not isinstance(item, dict):
            continue
        dimension = item.get("dimension")
        if dimension not in TASK_EFFECT_DIMENSION_SET:
            raise ValueError("Task 属性不合法")
        try:
            delta = round(float(item.get("delta", 0)), 1)
        except (TypeError, ValueError) as exc:
            raise ValueError("Task 数值不合法") from exc
        if delta == 0:
            continue
        merged[dimension] = round(merged.get(dimension, 0.0) + delta, 1)
    return [
        {"dimension": dimension, "delta": delta}
        for dimension, delta in merged.items()
        if delta != 0
    ]


class EventStore:
    """追加式 JSONL 事件流存储。所有状态都通过重放事件生成。"""

    def __init__(self, path: str | Path) -> None:
        self.path = Path(path)
        self.path.parent.mkdir(parents=True, exist_ok=True)
        self._lock = threading.RLock()
        self._events_cache: list[dict[str, Any]] | None = None
        self._events_cache_mtime: float | None = None
        if not self.path.exists():
            self.path.write_text("", encoding="utf-8")

    def _read_lines(self) -> list[str]:
        if not self.path.exists():
            return []
        with self.path.open("r", encoding="utf-8") as handle:
            return [line for line in handle.read().splitlines() if line.strip()]

    def _events_mtime(self) -> float | None:
        try:
            return self.path.stat().st_mtime
        except OSError:
            return None

    def _load_events_locked(self) -> list[dict[str, Any]]:
        """按 mtime 缓存解析结果：同一 mtime 内只解析一次 JSONL。"""
        mtime = self._events_mtime()
        if self._events_cache is None or mtime != self._events_cache_mtime:
            events: list[dict[str, Any]] = []
            for line in self._read_lines():
                try:
                    events.append(json.loads(line))
                except json.JSONDecodeError:
                    # 单行损坏不应阻断整个流；保留可读事件并继续。
                    continue
            self._events_cache = events
            self._events_cache_mtime = mtime
        return self._events_cache

    def read_events(self) -> list[dict[str, Any]]:
        with self._lock:
            return list(self._load_events_locked())

    def append_event(self, event: dict[str, Any]) -> dict[str, Any]:
        with self._lock:
            event.setdefault("event_id", str(uuid.uuid4()))
            event.setdefault("created_at", now_iso())
            event.setdefault("at", event["created_at"])
            serialized = json.dumps(event, ensure_ascii=False, separators=(",", ":"))
            with self.path.open("a", encoding="utf-8") as handle:
                handle.write(serialized + "\n")
            # 追加后让缓存失效，下次读取重新解析（外部直接改文件同理靠 mtime 失效）。
            self._events_cache = None
            return event

    def append_events(self, events: list[dict[str, Any]]) -> None:
        for event in events:
            self.append_event(event)

    def ensure_initialized(self) -> None:
        with self._lock:
            if self._read_lines():
                return
            today = iso_date()
            self.append_event(
                {
                    "type": "SYSTEM_INIT",
                    "name": "系统初始化",
                    "changes": [],
                    "date": today,
                    "at": now_iso(),
                }
            )
            self.append_event(
                {
                    "type": "EPIC_CREATED",
                    "id": "epic_infinite_progress",
                    "name": "创建里程碑：无限进步",
                    "changes": [],
                    "title": "无限进步",
                    "description": "系统默认里程碑。所有未明确归属的日常 Task 都会沉淀到这里。",
                    "main_dimension": "professional",
                    "title_emoji": "🚀",
                    "title_bonus_dimension": "professional",
                    "title_bonus_percent": 0,
                    "unlock_title_id": None,
                    "date": today,
                    "at": now_iso(),
                }
            )

    def last_tick_day(self) -> str | None:
        for event in reversed(self.read_events()):
            if event.get("type") == "SYSTEM_DAILY_TICK":
                return event.get("date")
        return None

    def ensure_daily_ticks(self) -> None:
        self.ensure_initialized()
        today = iso_date()
        with self._lock:
            last = self.last_tick_day()
            if last == today:
                return
            start = date.fromisoformat(last) + timedelta(days=1) if last else date.fromisoformat(today)
            end = date.fromisoformat(today)
            cursor = start
            while cursor <= end:
                self.append_event(
                    {
                        "type": "SYSTEM_DAILY_TICK",
                        "name": "每日结算",
                        "changes": [],
                        "date": cursor.isoformat(),
                        "at": datetime.now().astimezone().isoformat(timespec="seconds"),
                    }
                )
                cursor += timedelta(days=1)

    def _ensure_ready(self) -> None:
        """所有业务命令的统一前置：初始化事件流并补齐每日 Tick。"""
        self.ensure_initialized()
        self.ensure_daily_ticks()

    def get_state(self) -> dict[str, Any]:
        self._ensure_ready()
        events = self.read_events()
        return build_state(events)

    def create_epic(
        self,
        title: str,
        description: str,
        main_dimension: str,
        title_bonus_dimension: str,
        title_bonus_percent: float,
        title_emoji: str = "🏅",
    ) -> dict[str, Any]:
        self._ensure_ready()
        event = {
            "type": "EPIC_CREATED",
            "id": str(uuid.uuid4()),
            "name": f"创建里程碑：{title}",
            "changes": [],
            "title": title,
            "description": description,
            "main_dimension": main_dimension,
            "title_emoji": title_emoji,
            "title_bonus_dimension": title_bonus_dimension,
            "title_bonus_percent": title_bonus_percent,
            "date": iso_date(),
            "at": now_iso(),
        }
        self.append_event(event)
        return event

    def update_epic(
        self,
        epic_id: str,
        *,
        title: str,
        description: str,
        main_dimension: str,
        title_bonus_dimension: str,
        title_bonus_percent: float,
        title_emoji: str,
    ) -> dict[str, Any]:
        self._ensure_ready()
        state = self.get_state()
        epic = state["epics"].get(epic_id)
        if not epic:
            raise KeyError("里程碑不存在")
        if title_bonus_dimension not in TITLE_BONUS_DIMENSIONS:
            raise ValueError("称号加成属性不合法")
        new_title = title.strip()
        if not new_title:
            raise ValueError("请填写里程碑名称")
        event = {
            "type": "EPIC_UPDATED",
            "id": epic_id,
            "name": f"更新里程碑：{new_title}",
            "changes": [],
            "title": new_title,
            "description": description.strip(),
            "main_dimension": main_dimension,
            "title_bonus_dimension": title_bonus_dimension,
            "title_bonus_percent": float(title_bonus_percent),
            "title_emoji": title_emoji.strip() or "🏅",
            "date": iso_date(),
            "at": now_iso(),
        }
        self.append_event(event)
        return event

    def complete_epic(self, epic_id: str, engraving: str) -> dict[str, Any]:
        self._ensure_ready()
        state = self.get_state()
        epic = state["epics"].get(epic_id)
        if not epic:
            raise KeyError("里程碑不存在")
        if epic.get("status") == "completed":
            raise ValueError("里程碑已经完成")
        event = {
            "type": "EPIC_COMPLETED",
            "id": epic_id,
            "name": f"里程碑结项：{epic.get('title', '')}",
            "changes": [],
            "engraving": engraving,
            "date": iso_date(),
            "at": now_iso(),
        }
        self.append_event(event)
        return event

    def create_task(
        self,
        title: str,
        epic_id: str | None,
        *,
        effects: list[dict[str, Any]] | None = None,
        repeatable: bool = False,
        tags: list[str] | None = None,
    ) -> dict[str, Any]:
        self._ensure_ready()
        state = self.get_state()
        if epic_id:
            epic = state["epics"].get(epic_id)
            if not epic:
                raise KeyError("里程碑不存在")
        normalized_effects = _normalize_effects(effects)
        if not normalized_effects:
            raise ValueError("至少设置一个属性增益或减益")
        event = {
            "type": "TASK_CREATED",
            "id": str(uuid.uuid4()),
            "name": f"创建 Task：{title}",
            "changes": normalized_effects,
            "title": title,
            "epic_id": epic_id,
            "effects": normalized_effects,
            "repeatable": bool(repeatable),
            "tags": tags or [],
            "date": iso_date(),
            "at": now_iso(),
        }
        self.append_event(event)
        return event

    def complete_task(self, task_id: str, note: str = "") -> dict[str, Any]:
        self._ensure_ready()
        state = self.get_state()
        task = state["tasks"].get(task_id)
        if not task:
            raise KeyError("Task 不存在")
        if task.get("deleted") or task.get("status") == "deleted":
            raise ValueError("Task 已删除")
        if task.get("status") == "completed":
            raise ValueError("Task 已经完成")
        effects = apply_title_bonuses(
            task.get("effects") if isinstance(task.get("effects"), list) else [],
            state,
        )
        san_delta = round(
            sum(float(item.get("delta", 0)) for item in effects if item.get("dimension") == "san"),
            1,
        )
        current_san = float(state["dimensions"]["san"])
        if san_delta < 0 and current_san + san_delta < 0:
            raise ValueError(f"SAN 不足：当前 {current_san}，该 Task 需要 {abs(san_delta)}")
        event = {
            "type": "TASK_COMPLETED",
            "id": task_id,
            "name": f"记录 Task：{task.get('title', '')}",
            "changes": effects,
            "note": note,
            "effects": effects,
            "base_effects": task.get("effects", []),
            "date": iso_date(),
            "at": now_iso(),
        }
        self.append_event(event)
        return event

    def update_task(
        self,
        task_id: str,
        *,
        title: str | None = None,
        epic_id: str | None = None,
        effects: list[dict[str, Any]] | None = None,
        repeatable: bool | None = None,
    ) -> dict[str, Any]:
        self._ensure_ready()
        state = self.get_state()
        task = state["tasks"].get(task_id)
        if not task:
            raise KeyError("Task 不存在")
        if task.get("deleted") or task.get("status") == "deleted":
            raise ValueError("Task 已删除")
        if epic_id and epic_id not in state["epics"]:
            raise KeyError("里程碑不存在")
        normalized_effects = _normalize_effects(effects)
        if not normalized_effects:
            raise ValueError("至少保留一个属性增益或减益")
        new_title = (title or task.get("title") or "").strip()
        if not new_title:
            raise ValueError("请填写 Task 名称")
        if repeatable is None:
            repeatable = bool(task.get("repeatable"))
        event = {
            "type": "TASK_UPDATED",
            "id": task_id,
            "name": f"更新 Task：{new_title}",
            "changes": normalized_effects,
            "title": new_title,
            "epic_id": epic_id,
            "effects": normalized_effects,
            "repeatable": bool(repeatable),
            "date": iso_date(),
            "at": now_iso(),
        }
        self.append_event(event)
        return event

    def delete_task(self, task_id: str) -> dict[str, Any]:
        self._ensure_ready()
        state = self.get_state()
        task = state["tasks"].get(task_id)
        if not task:
            raise KeyError("Task 不存在")
        if task.get("deleted") or task.get("status") == "deleted":
            raise ValueError("Task 已删除")
        deleted_at = now_iso()
        event = {
            "type": "TASK_DELETED",
            "id": task_id,
            "name": f"删除 Task：{task.get('title', '')}",
            "changes": [],
            "deleted": True,
            "deleted_at": deleted_at,
            "date": iso_date(),
            "at": deleted_at,
        }
        self.append_event(event)
        return event

    def equip_title(self, title_id: str) -> dict[str, Any]:
        self._ensure_ready()
        state = self.get_state()
        title = state["titles"].get(title_id)
        if not title or not title.get("unlocked"):
            raise ValueError("称号尚未解锁")
        if title_id in state["equipped"]:
            raise ValueError("称号已经装备")
        if len(state["equipped"]) >= MAX_EQUIPPED_TITLES:
            raise ValueError("最多只能装备 3 个称号")
        event = {
            "type": "TITLE_EQUIPPED",
            "name": f"装备称号：{title_id}",
            "changes": [],
            "title_id": title_id,
            "date": iso_date(),
            "at": now_iso(),
        }
        self.append_event(event)
        return event

    def unequip_title(self, title_id: str) -> dict[str, Any]:
        self._ensure_ready()
        event = {
            "type": "TITLE_UNEQUIPPED",
            "name": f"卸下称号：{title_id}",
            "changes": [],
            "title_id": title_id,
            "date": iso_date(),
            "at": now_iso(),
        }
        self.append_event(event)
        return event

    def awaken(self) -> dict[str, Any]:
        self._ensure_ready()
        state = self.get_state()
        if state["profile"].get("awakened"):
            raise ValueError("已经觉醒")
        event = {"type": "PROFILE_AWAKENED", "name": "提前觉醒", "changes": [], "date": iso_date(), "at": now_iso()}
        self.append_event(event)
        return event

    def manual_tick(self) -> dict[str, Any]:
        self._ensure_ready()
        return {"type": "SYSTEM_DAILY_TICK", "date": iso_date(), "at": now_iso()}

    def delete_event(self, event_id: str, note: str = "") -> dict[str, Any]:
        """软删除一条事件：追加 tombstone，重放时忽略目标事件。"""
        self._ensure_ready()
        events = self.read_events()
        target = None
        for event in events:
            if event.get("event_id") == event_id or event.get("id") == event_id:
                target = event
                break
        if not target:
            raise KeyError("事件不存在")
        if target.get("type") in {"SYSTEM_INIT", "SYSTEM_DAILY_TICK", "EVENT_DELETED", "STREAM_CLEARED"}:
            raise ValueError("系统事件不允许删除")

        target_key = target.get("event_id") or target.get("id")
        for event in events:
            if event.get("type") == "EVENT_DELETED" and event.get("target_event_id") == target_key:
                raise ValueError("事件已经删除")

        deleted_at = now_iso()
        deletion = {
            "type": "EVENT_DELETED",
            "name": f"删除事件：{target.get('name') or target.get('title') or target.get('type')}",
            "changes": [],
            "target_event_id": target_key,
            "target_type": target.get("type"),
            "note": note,
            "deleted": True,
            "deleted_at": deleted_at,
            "date": iso_date(),
            "at": deleted_at,
        }
        self.append_event(deletion)
        return deletion
