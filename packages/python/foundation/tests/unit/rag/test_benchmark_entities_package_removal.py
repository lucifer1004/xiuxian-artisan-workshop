"""Removal guard for deleted benchmark-era skill runtime fixtures."""

from __future__ import annotations

import importlib

import pytest


@pytest.mark.parametrize(
    "module_name",
    [
        "agent.core.skill_runtime.context",
        "test_skills",
    ],
)
def test_deleted_benchmark_runtime_modules_are_unavailable(module_name: str) -> None:
    """Deleted benchmark/runtime helper modules must stay absent."""
    with pytest.raises(ModuleNotFoundError):
        importlib.import_module(module_name)
