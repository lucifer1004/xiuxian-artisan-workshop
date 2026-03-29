# dirs.py
"""
Project Directory Utilities.

Stable forwarding module for project path APIs.

Canonical implementations live in:
- `xiuxian_foundation.config.prj`
"""

from xiuxian_foundation.config.prj import (
    PRJ_CACHE,
    PRJ_CHECKPOINT,
    PRJ_CONFIG,
    PRJ_DATA,
    PRJ_DIRS,
    PRJ_PATH,
    PRJ_RUNTIME,
    get_cache_dir,
    get_config_dir,
    get_data_dir,
    get_prj_dir,
    get_runtime_dir,
    get_skills_dir,
)

__all__ = [
    "PRJ_CACHE",
    "PRJ_CHECKPOINT",
    "PRJ_CONFIG",
    "PRJ_DATA",
    "PRJ_DIRS",
    "PRJ_PATH",
    "PRJ_RUNTIME",
    "get_cache_dir",
    "get_config_dir",
    "get_data_dir",
    "get_prj_dir",
    "get_runtime_dir",
    "get_skills_dir",
]
