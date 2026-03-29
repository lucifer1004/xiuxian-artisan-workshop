"""Removal guard for deleted foundation runtime isolation helpers."""

from __future__ import annotations

from pathlib import Path


RUNTIME_ROOT = Path(__file__).resolve().parents[3] / "src" / "xiuxian_foundation" / "runtime"


def test_isolation_module_is_removed() -> None:
    """Foundation runtime should not retain local isolated script execution helpers."""
    assert not (RUNTIME_ROOT / "isolation.py").exists()


def test_isolation_not_exported_from_runtime() -> None:
    """Foundation runtime should not export the deleted isolation helper."""
    runtime_init = (RUNTIME_ROOT / "__init__.py").read_text(encoding="utf-8")
    assert "run_script_command" not in runtime_init
