"""Unit tests for knowledge benchmark YAML snapshot helpers."""

from __future__ import annotations

from typing import TYPE_CHECKING

import pytest
from omni.test_kit.knowledge_snapshot import (
    build_knowledge_snapshot_payload,
    detect_knowledge_snapshot_anomalies,
    load_knowledge_snapshot,
    save_knowledge_snapshot,
)

if TYPE_CHECKING:
    from pathlib import Path


def test_build_snapshot_payload_smooths_baseline_and_preserves_overrides() -> None:
    previous = {
        "tools": {
            "knowledge.recall": {
                "baseline_ms": 100.0,
                "last_avg_ms": 100.0,
                "regression_factor": 2.5,
                "min_regression_delta_ms": 70.0,
            }
        }
    }
    payload = build_knowledge_snapshot_payload(
        results=[{"tool": "knowledge.recall", "avg_ms": 200.0, "runs": 5, "ok": True}],
        runs_per_tool=5,
        warm_phase=True,
        previous=previous,
        alpha=0.25,
    )

    tool = payload["tools"]["knowledge.recall"]
    assert tool["baseline_ms"] == pytest.approx(125.0)
    assert tool["last_avg_ms"] == pytest.approx(200.0)
    assert tool["regression_factor"] == pytest.approx(2.5)
    assert tool["min_regression_delta_ms"] == pytest.approx(70.0)


def test_detect_snapshot_anomalies_uses_defaults_and_per_tool_overrides() -> None:
    snapshot = {
        "defaults": {"regression_factor": 2.0, "min_regression_delta_ms": 40.0},
        "tools": {
            "knowledge.recall": {"baseline_ms": 50.0},
            "knowledge.get_best_practice": {"baseline_ms": 50.0, "regression_factor": 3.0},
        },
    }
    results = [
        {"tool": "knowledge.recall", "avg_ms": 120.0, "ok": True},
        {"tool": "knowledge.get_best_practice", "avg_ms": 120.0, "ok": True},
    ]

    anomalies = detect_knowledge_snapshot_anomalies(results=results, snapshot=snapshot)
    assert len(anomalies) == 1
    assert anomalies[0].tool == "knowledge.recall"
    assert anomalies[0].threshold_ms == pytest.approx(100.0)


def test_save_and_load_snapshot_roundtrip(tmp_path: Path) -> None:
    path = tmp_path / "knowledge_tools.yaml"
    payload = {
        "schema": "omni.skills.knowledge_benchmark_snapshot.v1",
        "benchmark": {"runs_per_tool": 3, "warm_phase": True},
        "defaults": {"regression_factor": 2.0, "min_regression_delta_ms": 40.0},
        "tools": {"knowledge.recall": {"baseline_ms": 88.0, "last_avg_ms": 90.0}},
    }

    save_knowledge_snapshot(path, payload)
    loaded = load_knowledge_snapshot(path)
    assert loaded is not None
    assert loaded["schema"] == payload["schema"]
    assert loaded["tools"]["knowledge.recall"]["baseline_ms"] == pytest.approx(88.0)
