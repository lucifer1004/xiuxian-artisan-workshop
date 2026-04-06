"""Removal guard for the deleted runtime context package."""

from __future__ import annotations

from pathlib import Path


_RUNTIME_CONTEXT_DIR = (
    Path(__file__).resolve().parents[3] / "src" / "xiuxian_foundation" / "runtime" / "context"
)


def test_runtime_context_package_is_removed() -> None:
    assert not _RUNTIME_CONTEXT_DIR.exists()


def test_runtime_context_directory_is_physically_absent() -> None:
    assert _RUNTIME_CONTEXT_DIR.is_dir() is False
