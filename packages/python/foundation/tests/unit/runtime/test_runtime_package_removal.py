"""Removal guard for the deleted foundation runtime package."""

from __future__ import annotations

from pathlib import Path


FOUNDATION_ROOT = Path(__file__).resolve().parents[3] / "src" / "xiuxian_foundation"


def test_runtime_package_is_removed() -> None:
    """Foundation should no longer ship a standalone runtime package."""
    assert not (FOUNDATION_ROOT / "runtime").exists()
