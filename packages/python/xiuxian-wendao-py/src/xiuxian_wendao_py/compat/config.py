"""Self-contained config compatibility exports for the `xiuxian_*` namespace."""

from __future__ import annotations

import os
from dataclasses import dataclass
from pathlib import Path
from typing import Any

try:
    import yaml

    _YAML_AVAILABLE = True
except ImportError:
    _YAML_AVAILABLE = False


def _project_root() -> Path:
    prj_root = os.environ.get("PRJ_ROOT")
    if prj_root:
        return Path(prj_root)
    return Path.cwd()


def _env_dir(name: str, default: str) -> Path:
    return _project_root() / os.environ.get(name, default)


class _PrjDirsCallable:
    """Project directory accessor based on `PRJ_*` environment variables."""

    def clear_cache(self) -> None:
        """Compatibility no-op for callers that previously reset path caches."""
        return None

    @property
    def config_home(self) -> Path:
        return _env_dir("PRJ_CONFIG_HOME", ".config")

    @property
    def runtime_dir(self) -> Path:
        return _env_dir("PRJ_RUNTIME_DIR", ".run")

    @property
    def cache_home(self) -> Path:
        return _env_dir("PRJ_CACHE_HOME", ".cache")

    @property
    def data_home(self) -> Path:
        return _env_dir("PRJ_DATA_HOME", ".data")

    @property
    def path_dir(self) -> Path:
        return _env_dir("PRJ_PATH", ".bin")


class _PrjDirSingleton:
    """Callable singleton for one project directory family."""

    def __init__(self, env_name: str, default: str):
        self._env_name = env_name
        self._default = default

    def __call__(self, subdir: str = "", *more_parts: str) -> Path:
        path = _env_dir(self._env_name, self._default)
        if subdir:
            path = path / subdir
        for part in more_parts:
            path = path / part
        return path


PRJ_DIRS = _PrjDirsCallable()
PRJ_DATA = _PrjDirSingleton("PRJ_DATA_HOME", ".data")
PRJ_CACHE = _PrjDirSingleton("PRJ_CACHE_HOME", ".cache")
PRJ_CONFIG = _PrjDirSingleton("PRJ_CONFIG_HOME", ".config")
PRJ_RUNTIME = _PrjDirSingleton("PRJ_RUNTIME_DIR", ".run")
PRJ_PATH = _PrjDirSingleton("PRJ_PATH", ".bin")


def get_prj_dir(category: str = "data", subdir: str = "") -> Path:
    mapping = {
        "config": PRJ_CONFIG,
        "cache": PRJ_CACHE,
        "data": PRJ_DATA,
        "runtime": PRJ_RUNTIME,
        "path": PRJ_PATH,
    }
    factory = mapping.get(category, PRJ_DATA)
    return factory(subdir) if subdir else factory()


def get_data_dir(subdir: str = "") -> Path:
    return get_prj_dir("data", subdir)


def get_cache_dir(subdir: str = "") -> Path:
    return get_prj_dir("cache", subdir)


def get_config_dir(subdir: str = "") -> Path:
    return get_prj_dir("config", subdir)


def get_runtime_dir(subdir: str = "") -> Path:
    return get_prj_dir("runtime", subdir)


def get_skills_dir() -> Path:
    return _project_root() / "skills"


@dataclass(frozen=True, slots=True)
class ConfigPaths:
    """Minimal path bundle for legacy consumers migrating to `xiuxian_*`."""

    project_root: Path
    config_home: Path
    runtime_dir: Path
    cache_home: Path
    data_home: Path
    path_dir: Path

    def get_log_dir(self) -> Path:
        return self.runtime_dir / "logs"


def get_config_paths() -> ConfigPaths:
    return ConfigPaths(
        project_root=_project_root(),
        config_home=PRJ_DIRS.config_home,
        runtime_dir=PRJ_DIRS.runtime_dir,
        cache_home=PRJ_DIRS.cache_home,
        data_home=PRJ_DIRS.data_home,
        path_dir=PRJ_DIRS.path_dir,
    )


def reset_config_paths_cache() -> None:
    """Compatibility no-op for callers that previously reset ConfigPaths singletons."""
    return None


class XiuxianSettings:
    """Minimal settings loader for compatibility consumers."""

    def __init__(self) -> None:
        self._data = self._load()

    def _load_yaml(self, path: Path) -> dict[str, Any]:
        if not path.exists():
            return {}
        if _YAML_AVAILABLE:
            payload = yaml.safe_load(path.read_text(encoding="utf-8"))
            return payload or {}
        return {}

    def _deep_merge(self, base: dict[str, Any], override: dict[str, Any]) -> dict[str, Any]:
        result = dict(base)
        for key, value in override.items():
            if isinstance(result.get(key), dict) and isinstance(value, dict):
                result[key] = self._deep_merge(result[key], value)
            else:
                result[key] = value
        return result

    def _load(self) -> dict[str, Any]:
        conf_dir = _project_root() / "packages" / "conf"
        defaults = self._load_yaml(conf_dir / "settings.yaml")
        defaults = self._deep_merge(defaults, self._load_yaml(conf_dir / "wendao.yaml"))

        user_root = PRJ_CONFIG("xiuxian-artisan-workshop")
        user_settings = self._load_yaml(user_root / "settings.yaml")
        user_wendao = self._load_yaml(user_root / "wendao.yaml")
        return self._deep_merge(defaults, self._deep_merge(user_settings, user_wendao))

    def get(self, key: str, default: Any = None) -> Any:
        value: Any = self._data
        for part in key.split("."):
            if isinstance(value, dict) and part in value:
                value = value[part]
            else:
                return default
        return value


def get_settings() -> XiuxianSettings:
    return XiuxianSettings()


def get_setting(key: str, default: Any = None) -> Any:
    return get_settings().get(key, default)


__all__ = [
    "ConfigPaths",
    "PRJ_CACHE",
    "PRJ_CONFIG",
    "PRJ_DATA",
    "PRJ_DIRS",
    "PRJ_PATH",
    "PRJ_RUNTIME",
    "XiuxianSettings",
    "get_cache_dir",
    "get_config_dir",
    "get_config_paths",
    "get_data_dir",
    "get_prj_dir",
    "get_runtime_dir",
    "get_setting",
    "get_settings",
    "get_skills_dir",
    "reset_config_paths_cache",
]
