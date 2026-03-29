"""Removal guard for the deleted Python bridge package."""

from __future__ import annotations

import importlib
import importlib.util

import pytest


def test_bridge_package_is_removed() -> None:
    assert importlib.util.find_spec("xiuxian_foundation.bridge") is None


def test_bridge_package_import_raises_module_not_found() -> None:
    with pytest.raises(ModuleNotFoundError):
        importlib.import_module("xiuxian_foundation.bridge")
