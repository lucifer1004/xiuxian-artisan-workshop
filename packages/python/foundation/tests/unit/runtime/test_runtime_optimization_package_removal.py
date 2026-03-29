"""Removal guard for deleted foundation runtime optimization helpers."""

from __future__ import annotations

from pathlib import Path


RUNTIME_ROOT = Path(__file__).resolve().parents[3] / "src" / "xiuxian_foundation" / "runtime"


def test_runtime_optimization_module_is_removed() -> None:
    """Foundation runtime should not retain retrieval optimization helpers."""
    assert not (RUNTIME_ROOT / "runtime_optimization.py").exists()


def test_runtime_optimization_not_exported_from_runtime() -> None:
    """Foundation runtime should not export deleted retrieval optimization helpers."""
    runtime_init = (RUNTIME_ROOT / "__init__.py").read_text(encoding="utf-8")
    assert "normalize_chunk_window" not in runtime_init
    assert "filter_ranked_chunks" not in runtime_init
    assert "is_low_signal_query" not in runtime_init
