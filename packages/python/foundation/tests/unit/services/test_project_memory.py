"""Tests for ProjectMemory PRJ cache path resolution."""

from __future__ import annotations

import importlib

import xiuxian_foundation.config.prj as prj


def _reset_prj_cache() -> None:
    prj._project_root = None
    prj.PRJ_DIRS.clear_cache()


def test_project_memory_dir_uses_prj_cache_home(tmp_path, monkeypatch) -> None:
    cache_root = tmp_path / "cache-root"
    monkeypatch.setenv("PRJ_ROOT", str(tmp_path))
    monkeypatch.setenv("PRJ_CACHE_HOME", str(cache_root))
    _reset_prj_cache()

    from xiuxian_foundation.services.memory.core import project_memory

    module = importlib.reload(project_memory)

    try:
        assert module.MEMORY_DIR == cache_root / "memory"
    finally:
        _reset_prj_cache()
