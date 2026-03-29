"""Removal tests for legacy monitor surfaces."""

from __future__ import annotations

from pathlib import Path


def test_skills_monitor_package_removed() -> None:
    foundation_root = Path(__file__).resolve().parents[3] / "src" / "xiuxian_foundation"
    assert not (foundation_root / "runtime" / "skills_monitor.py").exists()


def test_runtime_package_no_longer_exports_skills_monitor_symbols() -> None:
    foundation_root = Path(__file__).resolve().parents[3] / "src" / "xiuxian_foundation"
    assert not (foundation_root / "runtime").exists()
