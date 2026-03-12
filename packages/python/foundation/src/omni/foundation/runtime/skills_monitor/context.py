"""Context and instrumentation for skills monitor."""

from __future__ import annotations

import contextvars
from typing import TYPE_CHECKING, Any

if TYPE_CHECKING:
    from .monitor import SkillsMonitor

_current_monitor: contextvars.ContextVar["SkillsMonitor | None"] = contextvars.ContextVar(
    "skills_monitor", default=None
)


def get_current_monitor() -> SkillsMonitor | None:
    """Return the active SkillsMonitor for the current context, if any."""
    return _current_monitor.get(None)


def set_current_monitor(monitor: SkillsMonitor | None) -> contextvars.Token:
    """Set the current monitor and return token for reset."""
    return _current_monitor.set(monitor)


def reset_current_monitor(token: contextvars.Token) -> None:
    """Restore previous monitor state."""
    _current_monitor.reset(token)


def record_phase(phase: str, duration_ms: float, **extra: Any) -> None:
    """Record a phase event to the current monitor (if active)."""
    mon = get_current_monitor()
    if mon is not None:
        mon.record_phase(phase, duration_ms, **extra)


def record_rust_db(op: str, duration_ms: float, **extra: Any) -> None:
    """Record a Rust/DB event to the current monitor (if active)."""
    mon = get_current_monitor()
    if mon is not None:
        mon.record_rust_db(op, duration_ms, **extra)
