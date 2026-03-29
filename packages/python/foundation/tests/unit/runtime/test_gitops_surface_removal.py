"""Removal guard for the deleted standalone gitops module."""

from __future__ import annotations

from pathlib import Path


def test_gitops_module_is_removed() -> None:
    foundation_root = Path(__file__).resolve().parents[3] / "src" / "xiuxian_foundation"
    assert not (foundation_root / "runtime" / "gitops.py").exists()
