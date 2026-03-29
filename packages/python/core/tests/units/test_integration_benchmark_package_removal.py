"""Removal guards for deleted benchmark and integration helper surfaces."""

from __future__ import annotations

from pathlib import Path


def _core_tests_root() -> Path:
    return Path(__file__).resolve().parents[1]


def test_benchmark_harness_is_deleted() -> None:
    benchmarks_dir = _core_tests_root() / "benchmarks"
    assert not (benchmarks_dir / "conftest.py").exists()


def test_legacy_integration_memory_helper_is_deleted() -> None:
    integration_dir = _core_tests_root() / "integration"
    assert not (integration_dir / "test_rust_bridge_config.py").exists()
