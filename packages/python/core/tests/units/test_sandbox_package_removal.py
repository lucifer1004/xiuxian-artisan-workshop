"""Removal guards for deleted Python sandbox bindings surfaces."""

from __future__ import annotations

from importlib.util import find_spec
from pathlib import Path


def test_python_sandbox_module_is_deleted() -> None:
    assert find_spec("xiuxian_core.sandbox") is None


def test_sandbox_bindings_test_is_deleted() -> None:
    units_dir = Path(__file__).resolve().parent
    assert not (units_dir / "test_sandbox.py").exists()
