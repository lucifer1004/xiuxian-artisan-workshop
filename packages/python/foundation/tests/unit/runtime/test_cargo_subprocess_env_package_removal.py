"""Removal guard for deleted local cargo subprocess env helpers."""

from __future__ import annotations

from pathlib import Path


FOUNDATION_ROOT = Path(__file__).resolve().parents[3] / "src" / "xiuxian_foundation"
RUNTIME_ROOT = FOUNDATION_ROOT / "runtime"


def test_cargo_subprocess_env_module_is_removed() -> None:
    """The local cargo subprocess helper should not remain in foundation runtime."""
    assert not RUNTIME_ROOT.exists() or not (RUNTIME_ROOT / "cargo_subprocess_env.py").exists()


def test_cargo_subprocess_env_not_exported_from_foundation_root() -> None:
    """Foundation root should not re-export the deleted cargo subprocess helper."""
    foundation_init = (FOUNDATION_ROOT / "__init__.py").read_text(encoding="utf-8")
    assert "prepare_cargo_subprocess_env" not in foundation_init
