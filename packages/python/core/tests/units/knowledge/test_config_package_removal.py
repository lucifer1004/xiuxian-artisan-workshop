from __future__ import annotations

import importlib

import pytest


def test_knowledge_config_module_removed() -> None:
    with pytest.raises(ModuleNotFoundError):
        importlib.import_module("xiuxian_core.knowledge.config")
