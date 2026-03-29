from __future__ import annotations

from pathlib import Path


def test_config_paths_module_is_deleted() -> None:
    paths_module = Path(__file__).resolve().parents[3] / "src/xiuxian_foundation/config/paths.py"
    assert not paths_module.exists()


def test_config_package_no_longer_exports_config_paths() -> None:
    import xiuxian_foundation.config as config

    assert not hasattr(config, "ConfigPaths")
    assert not hasattr(config, "get_config_paths")
