"""Removal guard for deleted local cargo subprocess env helpers."""

from __future__ import annotations

from pathlib import Path


RUNTIME_ROOT = Path(__file__).resolve().parents[3] / "src" / "xiuxian_foundation" / "runtime"


def test_cargo_subprocess_env_module_is_removed() -> None:
    """The local cargo subprocess helper should not remain in foundation runtime."""
    assert not (RUNTIME_ROOT / "cargo_subprocess_env.py").exists()


def test_cargo_subprocess_env_not_exported_from_runtime() -> None:
    """Foundation runtime should not expose the deleted local cargo subprocess helper."""
    runtime_init = (RUNTIME_ROOT / "__init__.py").read_text(encoding="utf-8")
    assert "prepare_cargo_subprocess_env" not in runtime_init
