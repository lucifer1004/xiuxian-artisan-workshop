# base.py
"""
Project memory surface.

Python-owned memory storage backends were removed. This module only preserves
the surface area needed to emit a clear runtime error plus the markdown utility
helpers that remain transport-agnostic.
"""

from xiuxian_foundation.services.memory.core.interface import (
    STORAGE_MODE_LANCE,
    StorageMode,
)
from xiuxian_foundation.services.memory.core.project_memory import (
    MEMORY_DIR,
    ProjectMemory,
    init_memory_dir,
)
from xiuxian_foundation.services.memory.core.utils import format_decision, parse_decision

__all__ = [
    "MEMORY_DIR",
    "STORAGE_MODE_LANCE",
    "ProjectMemory",
    "StorageMode",
    "format_decision",
    "init_memory_dir",
    "parse_decision",
]
