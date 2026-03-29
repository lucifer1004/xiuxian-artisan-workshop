"""Minimal project path resolution for the Python utility layer."""

from __future__ import annotations

from dataclasses import dataclass
from functools import lru_cache
from pathlib import Path


@dataclass(frozen=True, slots=True)
class ConfigPaths:
    """Resolved project-local paths used by injected Python helpers."""

    project_root: Path
    config_home: Path
    runtime_dir: Path
    cache_home: Path
    data_home: Path
    path_dir: Path

    @classmethod
    def create(cls) -> "ConfigPaths":
        """Build the current process path bundle from PRJ_* directories."""
        from .prj import PRJ_DIRS
        from ..runtime.gitops import get_project_root

        project_root = get_project_root()
        return cls(
            project_root=project_root,
            config_home=PRJ_DIRS.config_home,
            runtime_dir=PRJ_DIRS.runtime_dir,
            cache_home=PRJ_DIRS.cache_home,
            data_home=PRJ_DIRS.data_home,
            path_dir=PRJ_DIRS.path_dir,
        )

    def get_log_dir(self) -> Path:
        """Return the runtime log directory for injected consumers."""
        return self.runtime_dir / "logs"


@lru_cache(maxsize=1)
def get_config_paths() -> ConfigPaths:
    """Return cached project-local paths for the current process."""
    return ConfigPaths.create()


__all__ = ["ConfigPaths", "get_config_paths"]
