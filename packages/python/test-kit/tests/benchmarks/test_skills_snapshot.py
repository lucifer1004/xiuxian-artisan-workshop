"""Unit tests for skill-tools benchmark YAML snapshot helpers."""

from __future__ import annotations

from typing import TYPE_CHECKING

import pytest
from omni.test_kit.skills_snapshot import (
    build_skills_snapshot_payload,
    detect_skills_snapshot_anomalies,
    load_skills_snapshot,
    save_skills_snapshot,
)

if TYPE_CHECKING:
    from pathlib import Path


def test_build_snapshot_payload_smooths_baseline_and_preserves_overrides() -> None:
    previous = {
        "tools": {
            "crawl4ai.crawl_url": {
                "baseline_ms": 1200.0,
                "last_avg_ms": 1200.0,
                "regression_factor": 2.5,
                "min_regression_delta_ms": 200.0,
            }
        }
    }
    payload = build_skills_snapshot_payload(
        results=[{"tool": "crawl4ai.crawl_url", "avg_ms": 800.0, "runs": 3, "ok": True}],
        runs_per_tool=3,
        warm_phase=True,
        previous=previous,
        alpha=0.25,
    )

    tool = payload["tools"]["crawl4ai.crawl_url"]
    assert tool["baseline_ms"] == pytest.approx(1100.0)
    assert tool["last_avg_ms"] == pytest.approx(800.0)
    assert tool["last_p50_ms"] == pytest.approx(800.0)
    assert tool["last_p95_ms"] == pytest.approx(800.0)
    assert tool["regression_factor"] == pytest.approx(2.5)
    assert tool["min_regression_delta_ms"] == pytest.approx(200.0)


def test_build_snapshot_payload_uses_p50_for_baseline_when_present() -> None:
    payload = build_skills_snapshot_payload(
        results=[
            {
                "tool": "crawl4ai.crawl_url",
                "avg_ms": 700.0,
                "p50_ms": 480.0,
                "p95_ms": 980.0,
                "min_ms": 420.0,
                "max_ms": 1110.0,
                "stdev_ms": 145.0,
                "runs": 8,
                "ok": True,
                "scenario": "local_file",
            }
        ],
        runs_per_tool=8,
        warm_phase=True,
        previous=None,
    )

    tool = payload["tools"]["crawl4ai.crawl_url"]
    assert tool["baseline_ms"] == pytest.approx(480.0)
    assert tool["last_avg_ms"] == pytest.approx(700.0)
    assert tool["last_p50_ms"] == pytest.approx(480.0)
    assert tool["last_p95_ms"] == pytest.approx(980.0)
    assert tool["last_min_ms"] == pytest.approx(420.0)
    assert tool["last_max_ms"] == pytest.approx(1110.0)
    assert tool["last_stdev_ms"] == pytest.approx(145.0)
    assert tool["scenario"] == "local_file"


def test_detect_snapshot_anomalies_uses_defaults_and_per_tool_overrides() -> None:
    snapshot = {
        "defaults": {"regression_factor": 2.0, "min_regression_delta_ms": 40.0},
        "tools": {
            "memory.search_memory": {"baseline_ms": 80.0},
            "skill.discover": {"baseline_ms": 100.0, "regression_factor": 3.0},
        },
    }
    results = [
        {"tool": "memory.search_memory", "avg_ms": 190.0, "ok": True},
        {"tool": "skill.discover", "avg_ms": 280.0, "ok": True},
    ]

    anomalies = detect_skills_snapshot_anomalies(results=results, snapshot=snapshot)
    assert len(anomalies) == 1
    assert anomalies[0].tool == "memory.search_memory"
    assert anomalies[0].threshold_ms == pytest.approx(160.0)
    assert anomalies[0].observed_metric == "avg_ms"


def test_detect_snapshot_anomalies_prefers_p95_for_higher_run_counts() -> None:
    snapshot = {
        "defaults": {"regression_factor": 2.0, "min_regression_delta_ms": 40.0},
        "tools": {"crawl4ai.crawl_url": {"baseline_ms": 300.0}},
    }
    results = [
        {
            "tool": "crawl4ai.crawl_url",
            "avg_ms": 380.0,
            "p95_ms": 760.0,
            "runs": 10,
            "ok": True,
            "scenario": "network_http",
        }
    ]

    anomalies = detect_skills_snapshot_anomalies(results=results, snapshot=snapshot)
    assert len(anomalies) == 1
    assert anomalies[0].observed_metric == "p95_ms"
    assert anomalies[0].observed_ms == pytest.approx(760.0)
    assert anomalies[0].scenario == "network_http"


def test_detect_snapshot_anomalies_honors_explicit_override_metric() -> None:
    snapshot = {
        "defaults": {"regression_factor": 2.0, "min_regression_delta_ms": 40.0},
        "tools": {"crawl4ai.crawl_url": {"baseline_ms": 300.0}},
    }
    results = [
        {
            "tool": "crawl4ai.crawl_url",
            "avg_ms": 420.0,
            "p95_ms": 780.0,
            "trimmed_avg_ms": 410.0,
            "anomaly_observed_ms": 410.0,
            "anomaly_observed_metric": "trimmed_avg_ms",
            "runs": 10,
            "ok": True,
            "scenario": "network_http",
        }
    ]

    anomalies = detect_skills_snapshot_anomalies(results=results, snapshot=snapshot)
    assert len(anomalies) == 0


def test_save_and_load_snapshot_roundtrip(tmp_path: Path) -> None:
    path = tmp_path / "skills_tools.yaml"
    payload = {
        "schema": "omni.skills.tools_benchmark_snapshot.v1",
        "benchmark": {"runs_per_tool": 2, "warm_phase": True},
        "defaults": {"regression_factor": 2.0, "min_regression_delta_ms": 40.0},
        "tools": {"memory.search_memory": {"baseline_ms": 90.0, "last_avg_ms": 94.0}},
    }

    save_skills_snapshot(path, payload)
    loaded = load_skills_snapshot(path)
    assert loaded is not None
    assert loaded["schema"] == payload["schema"]
    assert loaded["tools"]["memory.search_memory"]["baseline_ms"] == pytest.approx(90.0)
