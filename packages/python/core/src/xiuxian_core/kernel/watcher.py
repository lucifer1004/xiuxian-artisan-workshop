"""Removal surface for Python-side file watching and hot reload.

Rust/Wendao owns file watching, reload orchestration, and index refresh. Python
no longer hosts a local watcher or hot-reload path.
"""

from __future__ import annotations

from dataclasses import dataclass
from enum import Enum
from typing import Any


_WATCHER_REMOVAL_MESSAGE = (
    "Python hot reload and file watcher support have been removed. Use "
    "Rust/Wendao over Arrow Flight for reload orchestration and file-driven "
    "index updates."
)


def _raise_removed() -> None:
    raise RuntimeError(_WATCHER_REMOVAL_MESSAGE)


class FileChangeType(str, Enum):
    CREATED = "created"
    MODIFIED = "modified"
    DELETED = "deleted"
    ERROR = "error"
    CHANGED = "changed"


@dataclass(slots=True)
class FileChangeEvent:
    event_type: FileChangeType
    path: str
    is_directory: bool = False

    @classmethod
    def from_tuple(cls, data: tuple[str, str]) -> "FileChangeEvent":
        return cls(event_type=FileChangeType(data[0]), path=data[1], is_directory=False)


class ReactiveSkillWatcher:
    """Removed Python local file watcher."""

    def __init__(self, *args: Any, **kwargs: Any) -> None:
        _ = (args, kwargs)
        _raise_removed()


__all__ = [
    "FileChangeEvent",
    "FileChangeType",
    "ReactiveSkillWatcher",
]
