"""API forwarding tests for xiuxian_foundation.config.dirs."""

from __future__ import annotations

from importlib.util import find_spec

from xiuxian_foundation.config import prj
from xiuxian_foundation.config import dirs as dirs_mod


def test_dirs_forwards_prj_symbols() -> None:
    assert dirs_mod.PRJ_DIRS is prj.PRJ_DIRS
    assert dirs_mod.PRJ_CONFIG is prj.PRJ_CONFIG
    assert dirs_mod.PRJ_DATA is prj.PRJ_DATA
    assert dirs_mod.PRJ_CACHE is prj.PRJ_CACHE
    assert dirs_mod.PRJ_RUNTIME is prj.PRJ_RUNTIME
    assert dirs_mod.get_prj_dir is prj.get_prj_dir
    assert dirs_mod.get_config_dir is prj.get_config_dir
    assert dirs_mod.get_data_dir is prj.get_data_dir
    assert dirs_mod.get_cache_dir is prj.get_cache_dir
    assert dirs_mod.get_runtime_dir is prj.get_runtime_dir
    assert dirs_mod.get_skills_dir is prj.get_skills_dir


def test_dirs_no_longer_forward_removed_symbols() -> None:
    assert find_spec("xiuxian_foundation.config.database") is None
    assert find_spec("xiuxian_foundation.config.harvested") is None
    assert not hasattr(dirs_mod, "get_vector_db_path")
    assert not hasattr(dirs_mod, "get_memory_db_path")
    assert not hasattr(dirs_mod, "get_harvest_dir")
    assert not hasattr(dirs_mod, "get_harvest_file")
