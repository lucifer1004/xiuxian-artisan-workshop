"""Removal checks for deleted config directory/harvested helpers."""

from __future__ import annotations

from importlib.util import find_spec

import xiuxian_foundation.config.dirs as foundation_dirs


def test_removed_config_helper_packages_are_absent() -> None:
    assert find_spec("xiuxian_foundation.config.directory") is None
    assert find_spec("xiuxian_foundation.config.harvested") is None


def test_removed_config_helpers_are_not_exported() -> None:
    assert not hasattr(foundation_dirs, "get_conf_dir")
    assert not hasattr(foundation_dirs, "set_conf_dir")
    assert not hasattr(foundation_dirs, "get_harvest_dir")
    assert not hasattr(foundation_dirs, "get_harvest_file")
