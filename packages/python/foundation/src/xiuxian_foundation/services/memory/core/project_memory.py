"""ProjectMemory removal surface.

Python-owned memory storage backends were removed. Memory persistence now
belongs to Rust-owned services reached over Arrow Flight or other Rust-managed
runtime boundaries.
"""

from __future__ import annotations

from pathlib import Path

import structlog

from xiuxian_foundation.services.memory.core.interface import STORAGE_MODE_LANCE, StorageMode

log = structlog.get_logger("xiuxian_foundation.memory")


def _get_memory_dir() -> Path:
    """Get memory directory from PRJ_CACHE_HOME."""
    from xiuxian_foundation.config.dirs import PRJ_CACHE

    return PRJ_CACHE("memory")


MEMORY_DIR = _get_memory_dir()


def init_memory_dir(dir_path: Path | None = None) -> bool:
    """Initialize the local memory artifact directory structure."""
    if dir_path is None:
        dir_path = MEMORY_DIR

    try:
        dir_path.mkdir(parents=True, exist_ok=True)
        (dir_path / "decisions").mkdir(exist_ok=True)
        (dir_path / "tasks").mkdir(exist_ok=True)
        (dir_path / "context").mkdir(exist_ok=True)
        (dir_path / "active_context").mkdir(exist_ok=True)
        return True
    except Exception as exc:
        log.info("memory.init_failed", error=str(exc))
        return False


class ProjectMemory:
    """Removed Python project-memory surface."""

    def __init__(self, dir_path: Path | None = None):
        del dir_path
        raise RuntimeError(
            "Python local memory storage was removed; use Rust-owned memory services "
            "over Arrow Flight instead of ProjectMemory."
        )

    @property
    def storage_mode(self) -> StorageMode:
        """Expose the removed legacy storage mode for compatibility."""
        return STORAGE_MODE_LANCE

    @property
    def is_lance_mode(self) -> bool:
        """Expose the removed legacy backend marker for compatibility."""
        return True


__all__ = ["MEMORY_DIR", "ProjectMemory", "init_memory_dir"]
