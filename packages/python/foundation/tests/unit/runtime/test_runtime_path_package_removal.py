"""Removal guard for the deleted runtime path helper."""

from __future__ import annotations

from pathlib import Path


_RUNTIME_PATH_FILE = (
    Path(__file__).resolve().parents[3] / "src" / "xiuxian_foundation" / "runtime" / "path.py"
)


def test_runtime_path_module_is_removed() -> None:
    assert not _RUNTIME_PATH_FILE.exists()


def test_runtime_path_file_is_physically_absent() -> None:
    assert _RUNTIME_PATH_FILE.is_file() is False
