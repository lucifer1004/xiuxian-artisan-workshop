"""Removal guard for the deleted path-filter helper."""

from __future__ import annotations

from pathlib import Path


def test_runtime_path_filter_module_is_removed() -> None:
    foundation_root = Path(__file__).resolve().parents[3] / "src" / "xiuxian_foundation"
    assert not (foundation_root / "runtime" / "path_filter.py").exists()


def test_runtime_package_directory_is_removed() -> None:
    foundation_root = Path(__file__).resolve().parents[3] / "src" / "xiuxian_foundation"
    assert not (foundation_root / "runtime").exists()
