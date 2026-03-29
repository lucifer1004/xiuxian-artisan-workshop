from __future__ import annotations

import importlib

import pytest


def test_core_context_tools_module_removed() -> None:
    with pytest.raises(ModuleNotFoundError):
        importlib.import_module("xiuxian_core.context.tools")
