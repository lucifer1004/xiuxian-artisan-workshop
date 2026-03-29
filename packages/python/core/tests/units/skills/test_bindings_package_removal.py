"""Removal tests for retired Python-side scanner and vector binding surfaces."""

from __future__ import annotations

import importlib

import pytest


def test_tools_loader_index_module_is_removed() -> None:
    with pytest.raises(ModuleNotFoundError):
        importlib.import_module("xiuxian_core.skills.tools_loader_index")


def test_rust_scanner_module_is_removed() -> None:
    with pytest.raises(ModuleNotFoundError):
        importlib.import_module("xiuxian_foundation.bridge.rust_scanner")


def test_bridge_package_does_not_export_rust_skill_scanner() -> None:
    import xiuxian_foundation.bridge as bridge

    assert not hasattr(bridge, "RustSkillScanner")
    assert not hasattr(bridge, "is_rust_available")
    assert not hasattr(bridge, "check_rust_availability")
