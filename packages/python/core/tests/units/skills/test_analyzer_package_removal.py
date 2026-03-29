from __future__ import annotations

import importlib

import pytest


def test_skills_analyzer_package_removed() -> None:
    with pytest.raises(ModuleNotFoundError):
        importlib.import_module("xiuxian_core.skills.analyzer")
