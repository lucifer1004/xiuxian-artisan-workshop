"""Public API for xiuxian-wendao-py."""

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
    "WendaoRuntimeConfig",
    "WendaoTransportClient",
    "WendaoTransportConfig",
    "WendaoTransportEndpoint",
    "WendaoTransportMode",
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
    "ConfigPaths": (".compat", "ConfigPaths"),
    "PRJ_CACHE": (".compat", "PRJ_CACHE"),
    "PRJ_CONFIG": (".compat", "PRJ_CONFIG"),
    "PRJ_DATA": (".compat", "PRJ_DATA"),
    "PRJ_DIRS": (".compat", "PRJ_DIRS"),
    "PRJ_PATH": (".compat", "PRJ_PATH"),
    "PRJ_RUNTIME": (".compat", "PRJ_RUNTIME"),
    "WendaoRuntimeConfig": (".models", "WendaoRuntimeConfig"),
    "WendaoTransportClient": (".transport", "WendaoTransportClient"),
    "WendaoTransportConfig": (".transport", "WendaoTransportConfig"),
    "WendaoTransportEndpoint": (".transport", "WendaoTransportEndpoint"),
    "WendaoTransportMode": (".transport", "WendaoTransportMode"),
    "get_cache_dir": (".compat", "get_cache_dir"),
    "get_config_dir": (".compat", "get_config_dir"),
    "get_config_paths": (".compat", "get_config_paths"),
    "get_data_dir": (".compat", "get_data_dir"),
    "get_prj_dir": (".compat", "get_prj_dir"),
    "get_project_root": (".compat", "get_project_root"),
    "get_runtime_dir": (".compat", "get_runtime_dir"),
    "get_setting": (".compat", "get_setting"),
    "get_settings": (".compat", "get_settings"),
    "get_skills_dir": (".compat", "get_skills_dir"),
}


def __getattr__(name: str) -> Any:
    """Lazy-load public exports so transport imports do not pull in unrelated code."""
    try:
        module_name, symbol_name = _EXPORT_MAP[name]
    except KeyError as exc:
        raise AttributeError(f"module {__name__!r} has no attribute {name!r}") from exc

    module = import_module(module_name, __name__)
    return getattr(module, symbol_name)
