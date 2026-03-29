# utils
"""
Utilities Module

Provides common utility functions:
- templating.py: Template rendering
- skills.py: Skills-tree path helpers

Usage:
    from xiuxian_foundation.utils.templating import render_template
    from xiuxian_foundation.config.prj import get_skills_dir
"""

from .asyncio import run_async_blocking
from .fs import find_files_by_extension, find_markdown_files
from .json_codec import JSONDecodeError, dump as json_dump, dumps as json_dumps
from .json_codec import load as json_load, loads as json_loads
from .skills import (
    skill_asset,
    skill_data,
    skill_reference,
    skill_script,
)
from .templating import render_string

__all__ = [
    "find_files_by_extension",
    "find_markdown_files",
    "JSONDecodeError",
    "json_dump",
    "json_dumps",
    "json_load",
    "json_loads",
    "render_string",
    "run_async_blocking",
    "skill_asset",
    "skill_data",
    "skill_reference",
    "skill_script",
]
