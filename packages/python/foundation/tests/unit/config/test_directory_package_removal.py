"""Removal checks for deleted config directory/harvested helpers."""

from __future__ import annotations

from importlib.util import find_spec

from xiuxian_foundation import config as foundation_config
from xiuxian_foundation.config import dirs as foundation_dirs


def test_removed_config_helper_packages_are_absent() -> None:
    assert find_spec("xiuxian_foundation.config.directory") is None
    assert find_spec("xiuxian_foundation.config.harvested") is None


def test_removed_config_helpers_are_not_exported() -> None:
    for target in (foundation_config, foundation_dirs):
        assert not hasattr(target, "get_conf_dir")
        assert not hasattr(target, "set_conf_dir")
        assert not hasattr(target, "get_harvest_dir")
        assert not hasattr(target, "get_harvest_file")
