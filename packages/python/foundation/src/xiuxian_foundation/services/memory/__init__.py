# memory - Project Memory Persistence Module

"""
Project Memory Persistence Module.

Modules:
- base.py: ProjectMemory removal surface and markdown utilities
- core: Core interfaces, types, and utilities
- stores: retired Python storage namespace

Usage:
    from xiuxian_foundation.services.memory import ProjectMemory

    ProjectMemory()  # raises: local Python memory backends were removed
"""

from .base import (
    MEMORY_DIR,
    STORAGE_MODE_LANCE,
    ProjectMemory,
    StorageMode,
    format_decision,
    init_memory_dir,
    parse_decision,
)

__all__ = [
    "MEMORY_DIR",
    "STORAGE_MODE_LANCE",
    "ProjectMemory",
    "StorageMode",
    "format_decision",
    "init_memory_dir",
    "parse_decision",
]
