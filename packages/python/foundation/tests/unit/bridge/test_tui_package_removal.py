"""Removal checks for the deleted Python TUI bindings bridge."""

from __future__ import annotations

import importlib

import pytest


def test_tui_bridge_module_removed() -> None:
    with pytest.raises(ModuleNotFoundError):
        importlib.import_module("xiuxian_foundation.bridge.tui")
