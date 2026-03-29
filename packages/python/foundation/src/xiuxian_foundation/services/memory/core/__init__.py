# core/__init__.py
"""
Memory core module.

Provides interfaces, data types, and the ProjectMemory removal surface.

Submodules:
- interface: Abstract interfaces and data types
- project_memory: Main ProjectMemory implementation
- utils: Shared utility functions
"""

from xiuxian_foundation.services.memory.core.interface import STORAGE_MODE_LANCE, StorageMode
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
