"""Temporary compatibility exports under the canonical `xiuxian_*` namespace."""

from __future__ import annotations

from importlib import import_module
from typing import Any

__all__ = [
    "ConfigPaths",
    "PRJ_CACHE",
    "PRJ_CONFIG",
    "PRJ_DATA",
    "PRJ_DIRS",
    "PRJ_PATH",
    "PRJ_RUNTIME",
    "get_cache_dir",
    "get_config_dir",
    "get_config_paths",
    "get_data_dir",
    "get_prj_dir",
    "get_runtime_dir",
    "get_setting",
    "get_settings",
    "get_skills_dir",
    "get_project_root",
]

_EXPORT_MAP: dict[str, tuple[str, str]] = {
    "ConfigPaths": (".config", "ConfigPaths"),
    "PRJ_CACHE": (".config", "PRJ_CACHE"),
    "PRJ_CONFIG": (".config", "PRJ_CONFIG"),
    "PRJ_DATA": (".config", "PRJ_DATA"),
    "PRJ_DIRS": (".config", "PRJ_DIRS"),
    "PRJ_PATH": (".config", "PRJ_PATH"),
    "PRJ_RUNTIME": (".config", "PRJ_RUNTIME"),
    "get_cache_dir": (".config", "get_cache_dir"),
    "get_config_dir": (".config", "get_config_dir"),
    "get_config_paths": (".config", "get_config_paths"),
    "get_data_dir": (".config", "get_data_dir"),
    "get_prj_dir": (".config", "get_prj_dir"),
    "get_runtime_dir": (".config", "get_runtime_dir"),
    "get_setting": (".config", "get_setting"),
    "get_settings": (".config", "get_settings"),
    "get_skills_dir": (".config", "get_skills_dir"),
    "get_project_root": (".runtime", "get_project_root"),
}


def __getattr__(name: str) -> Any:
    """Lazy-load compatibility exports to avoid touching unrelated legacy chains."""
    try:
        module_name, symbol_name = _EXPORT_MAP[name]
    except KeyError as exc:
        raise AttributeError(f"module {__name__!r} has no attribute {name!r}") from exc

    module = import_module(module_name, __name__)
    return getattr(module, symbol_name)
