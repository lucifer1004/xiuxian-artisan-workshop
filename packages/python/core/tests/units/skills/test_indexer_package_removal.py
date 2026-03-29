from __future__ import annotations

import importlib

import pytest


def test_skill_indexer_module_is_deleted() -> None:
    with pytest.raises(ModuleNotFoundError):
        importlib.import_module("xiuxian_core.skills.indexer")
