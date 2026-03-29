"""Removal tests for the retired xiuxian_cell runtime surface."""

from __future__ import annotations

import importlib

import pytest


def test_xiuxian_cell_module_is_removed() -> None:
    with pytest.raises(ModuleNotFoundError):
        importlib.import_module("xiuxian_core.skills.runtime.xiuxian_cell")
