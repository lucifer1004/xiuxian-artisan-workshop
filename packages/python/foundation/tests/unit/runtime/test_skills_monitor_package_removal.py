"""Removal tests for legacy runtime skills monitor surfaces."""

from __future__ import annotations

import importlib
import importlib.util


def test_skills_monitor_package_removed() -> None:
    importlib.invalidate_caches()
    assert importlib.util.find_spec("xiuxian_foundation.runtime.skills_monitor") is None


def test_runtime_package_no_longer_exports_skills_monitor_symbols() -> None:
    import xiuxian_foundation.runtime as runtime

    assert not hasattr(runtime, "get_current_monitor")
    assert not hasattr(runtime, "record_phase")
    assert not hasattr(runtime, "record_rust_db")
    assert not hasattr(runtime, "run_with_monitor")
    assert not hasattr(runtime, "skills_monitor_scope")
