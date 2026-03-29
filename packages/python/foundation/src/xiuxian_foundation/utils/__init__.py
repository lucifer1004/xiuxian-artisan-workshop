# utils
"""
Utilities Module

Provides common utility functions:
- templating.py: Template rendering
- skills.py: Skill-related utilities
- common.py: Common helper functions

Usage:
    from xiuxian_foundation.utils.templating import render_template
    from xiuxian_foundation.config.skills import SKILLS_DIR
    from xiuxian_foundation.utils.common import is_binary
"""

from .common import project_root
from .asyncio import run_async_blocking
from .fs import find_files_by_extension, find_markdown_files
from .json_codec import JSONDecodeError, dump as json_dump, dumps as json_dumps
from .json_codec import load as json_load, loads as json_loads
from .skills import (
    current_skill_dir,
    skill_asset,
    skill_command,
    skill_data,
    skill_path,
    skill_reference,
)
from .templating import render_string

__all__ = [
    "current_skill_dir",
    "find_files_by_extension",
    "find_markdown_files",
    "JSONDecodeError",
    "json_dump",
    "json_dumps",
    "json_load",
    "json_loads",
    "project_root",
    "render_string",
    "run_async_blocking",
    "skill_asset",
    "skill_command",
    "skill_data",
    "skill_path",
    "skill_reference",
]
