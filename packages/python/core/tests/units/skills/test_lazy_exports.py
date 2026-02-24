"""Lazy export behavior tests for ``omni.core.skills`` package API."""

from __future__ import annotations

import importlib
import sys


def test_core_skills_import_does_not_eager_load_heavy_submodules() -> None:
    """Importing package root should not pull full subsystem graph eagerly."""
    for name in (
        "omni.core.skills",
        "omni.core.skills.discovery",
        "omni.core.skills.memory",
        "omni.core.skills.registry",
        "omni.core.skills.runtime",
        "omni.core.skills.universal",
    ):
        sys.modules.pop(name, None)

    importlib.import_module("omni.core.skills")

    assert "omni.core.skills.discovery" not in sys.modules
    assert "omni.core.skills.memory" not in sys.modules
    assert "omni.core.skills.registry" not in sys.modules
    assert "omni.core.skills.runtime" not in sys.modules
    assert "omni.core.skills.universal" not in sys.modules


def test_core_skills_lazy_export_loads_and_caches_target_symbol() -> None:
    """Accessing lazy symbol should import only its target module and cache value."""
    for name in (
        "omni.core.skills",
        "omni.core.skills.runner",
    ):
        sys.modules.pop(name, None)

    skills = importlib.import_module("omni.core.skills")
    first = skills.run_skill
    second = skills.run_skill

    assert callable(first)
    assert first is second
    assert "omni.core.skills.runner" in sys.modules
