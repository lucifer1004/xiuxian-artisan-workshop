"""Removal tests for legacy skills monitor schema surfaces."""

from __future__ import annotations

import importlib
import importlib.util


def test_skills_monitor_schema_module_removed() -> None:
    importlib.invalidate_caches()
    assert importlib.util.find_spec("xiuxian_foundation.api.skills_monitor_signals_schema") is None
