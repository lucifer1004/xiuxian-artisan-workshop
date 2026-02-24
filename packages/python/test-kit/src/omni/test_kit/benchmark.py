"""Scale benchmark helpers: latency thresholds and assertions.

Used by benchmark tests in test-kit to avoid regression on skills/core paths.
Run benchmark tests with: just test-benchmarks (from repo root).
"""

from __future__ import annotations

import os
import re
import time
from datetime import UTC, datetime
from math import ceil
from typing import TYPE_CHECKING, Any

from omni.foundation.config.dirs import PRJ_DIRS
from omni.foundation.utils import json_codec as json

if TYPE_CHECKING:
    from collections.abc import Awaitable, Callable
    from pathlib import Path


def benchmark_index_path(root_dir: Path | None = None) -> Path:
    """Return benchmark index path under ``$PRJ_RUNTIME_DIR/benchmarks``."""
    base_dir = root_dir or (PRJ_DIRS.runtime_dir / "benchmarks")
    return base_dir / "index.json"


def _percentile_nearest_rank(values: list[float], percentile: float) -> float:
    """Return nearest-rank percentile (e.g., P95)."""
    if not values:
        return 0.0
    if percentile <= 0:
        return min(values)
    if percentile >= 100:
        return max(values)
    ordered = sorted(values)
    rank = max(1, ceil((percentile / 100.0) * len(ordered)))
    return float(ordered[rank - 1])


def _as_float(value: object) -> float | None:
    """Return float when value is numeric, otherwise None."""
    if isinstance(value, bool):
        return None
    if isinstance(value, int | float):
        return float(value)
    return None


def _round_metric(value: float) -> float:
    """Round metric value to 3 decimals for stable JSON artifacts."""
    return round(float(value), 3)


def _extract_report(payload: dict[str, Any]) -> dict[str, Any] | None:
    """Extract monitor report dict from artifact payload."""
    if isinstance(payload.get("report"), dict):
        return payload["report"]
    # Support older direct-report artifacts.
    if isinstance(payload.get("skill_command"), str) and isinstance(payload.get("phases"), list):
        return payload
    return None


def _summarize_suite_artifacts(files: list[Path]) -> dict[str, Any]:
    """Build aggregate metrics for one benchmark suite directory."""
    elapsed_ms_values: list[float] = []
    rss_peak_delta_values: list[float] = []
    phase_stats: dict[str, dict[str, float]] = {}
    valid_reports = 0

    for artifact in files:
        try:
            raw_obj = json.loads(artifact.read_text(encoding="utf-8"))
        except Exception:
            continue
        if not isinstance(raw_obj, dict):
            continue
        report = _extract_report(raw_obj)
        if not isinstance(report, dict):
            continue

        valid_reports += 1
        elapsed_sec = _as_float(report.get("elapsed_sec"))
        if elapsed_sec is not None:
            elapsed_ms_values.append(elapsed_sec * 1000.0)

        rss_peak_delta: float | None = None
        if isinstance(report.get("rss_peak_mb"), dict):
            rss_peak_delta = _as_float(report["rss_peak_mb"].get("delta"))
        if rss_peak_delta is None:
            rss_peak_delta = _as_float(report.get("rss_peak_delta_mb"))
        if rss_peak_delta is not None:
            rss_peak_delta_values.append(rss_peak_delta)

        phases = report.get("phases")
        if not isinstance(phases, list):
            continue
        for phase in phases:
            if not isinstance(phase, dict):
                continue
            phase_name = phase.get("phase")
            if not isinstance(phase_name, str) or not phase_name:
                continue
            duration_ms = _as_float(phase.get("duration_ms"))
            if duration_ms is None:
                continue
            bucket = phase_stats.setdefault(
                phase_name,
                {"count": 0.0, "total_ms": 0.0, "max_ms": 0.0, "durations": []},  # type: ignore[list-item]
            )
            bucket["count"] += 1.0
            bucket["total_ms"] += duration_ms
            bucket["max_ms"] = max(bucket["max_ms"], duration_ms)
            durations = bucket.get("durations")
            if isinstance(durations, list):
                durations.append(duration_ms)

    latest_artifact = max(files, key=lambda p: p.stat().st_mtime) if files else None
    elapsed_summary = {
        "avg": _round_metric(sum(elapsed_ms_values) / len(elapsed_ms_values))
        if elapsed_ms_values
        else 0.0,
        "p50": _round_metric(_percentile_nearest_rank(elapsed_ms_values, 50.0)),
        "p95": _round_metric(_percentile_nearest_rank(elapsed_ms_values, 95.0)),
        "max": _round_metric(max(elapsed_ms_values)) if elapsed_ms_values else 0.0,
    }
    rss_peak_summary = {
        "avg": _round_metric(sum(rss_peak_delta_values) / len(rss_peak_delta_values))
        if rss_peak_delta_values
        else 0.0,
        "max": _round_metric(max(rss_peak_delta_values)) if rss_peak_delta_values else 0.0,
    }

    top_phases: list[dict[str, Any]] = []
    sortable: list[tuple[str, dict[str, float]]] = []
    for phase_name, stats in phase_stats.items():
        sortable.append((phase_name, stats))
    sortable.sort(key=lambda item: (-item[1]["total_ms"], item[0]))
    for phase_name, stats in sortable[:10]:
        durations = stats.get("durations")
        values = durations if isinstance(durations, list) else []
        top_phases.append(
            {
                "phase": phase_name,
                "count": int(stats["count"]),
                "total_ms": _round_metric(stats["total_ms"]),
                "avg_ms": _round_metric(stats["total_ms"] / stats["count"]),
                "p95_ms": _round_metric(_percentile_nearest_rank(values, 95.0)),
                "max_ms": _round_metric(stats["max_ms"]),
            }
        )

    return {
        "artifacts": len(files),
        "reports": valid_reports,
        "latest_artifact": str(latest_artifact) if latest_artifact else "",
        "elapsed_ms": elapsed_summary,
        "rss_peak_delta_mb": rss_peak_summary,
        "top_phases": top_phases,
    }


def refresh_benchmark_index(root_dir: Path | None = None) -> Path:
    """Scan benchmark artifacts and write aggregated index JSON."""
    base_dir = root_dir or (PRJ_DIRS.runtime_dir / "benchmarks")
    base_dir.mkdir(parents=True, exist_ok=True)

    suites: dict[str, dict[str, Any]] = {}
    for suite_dir in sorted(base_dir.iterdir()):
        if not suite_dir.is_dir():
            continue
        files = sorted(
            path for path in suite_dir.iterdir() if path.is_file() and path.suffix == ".json"
        )
        if not files:
            continue
        suites[suite_dir.name] = _summarize_suite_artifacts(files)

    totals = {
        "suites": len(suites),
        "artifacts": sum(item.get("artifacts", 0) for item in suites.values()),
        "reports": sum(item.get("reports", 0) for item in suites.values()),
    }
    payload = {
        "generated_at": datetime.now(UTC).isoformat(),
        "root": str(base_dir),
        "totals": totals,
        "suites": suites,
    }
    index_path = benchmark_index_path(base_dir)
    index_path.write_text(
        json.dumps(payload, indent=2, ensure_ascii=False) + "\n", encoding="utf-8"
    )
    return index_path


def assert_sync_latency_under_ms[T](
    fn: Callable[[], T],
    threshold_ms: float,
    iterations: int = 5,
) -> T:
    """Run sync fn N times; assert average latency < threshold_ms. Returns last result."""
    latencies: list[float] = []
    last: Any = None
    for _ in range(iterations):
        start = time.perf_counter()
        last = fn()
        latencies.append((time.perf_counter() - start) * 1000)
    avg_ms = sum(latencies) / len(latencies)
    assert avg_ms < threshold_ms, (
        f"Average latency {avg_ms:.1f}ms exceeds threshold {threshold_ms}ms (n={iterations})"
    )
    return last  # type: ignore[return-value]


async def assert_async_latency_under_ms[T](
    coro_fn: Callable[[], Awaitable[T]],
    threshold_ms: float,
    iterations: int = 5,
) -> T:
    """Run async coro_fn N times; assert average latency < threshold_ms. Returns last result."""
    latencies: list[float] = []
    last: Any = None
    for _ in range(iterations):
        start = time.perf_counter()
        last = await coro_fn()
        latencies.append((time.perf_counter() - start) * 1000)
    avg_ms = sum(latencies) / len(latencies)
    assert avg_ms < threshold_ms, (
        f"Average latency {avg_ms:.1f}ms exceeds threshold {threshold_ms}ms (n={iterations})"
    )
    return last  # type: ignore[return-value]


def dump_skills_monitor_report(
    report: Any,
    *,
    test_name: str,
    suite: str = "skills",
    metadata: dict[str, Any] | None = None,
) -> Path:
    """Write a skills-monitor report artifact to ``$PRJ_RUNTIME_DIR/benchmarks``.

    This helper is intended for benchmark/tests only.
    """
    if hasattr(report, "to_dict") and callable(report.to_dict):
        payload: dict[str, Any] = report.to_dict()
    elif isinstance(report, dict):
        payload = dict(report)
    else:
        payload = {"report": str(report)}

    if metadata:
        payload = {"metadata": dict(metadata), "report": payload}

    safe_test_name = re.sub(r"[^a-zA-Z0-9_.-]+", "_", test_name).strip("._")
    if not safe_test_name:
        safe_test_name = "benchmark"
    safe_suite = re.sub(r"[^a-zA-Z0-9_.-]+", "_", suite).strip("._")
    if not safe_suite:
        safe_suite = "skills"

    out_dir = PRJ_DIRS.runtime_dir / "benchmarks" / safe_suite
    out_dir.mkdir(parents=True, exist_ok=True)
    stamp = datetime.now(UTC).strftime("%Y%m%dT%H%M%S.%fZ")
    out_path = out_dir / f"{safe_test_name}.{stamp}.{os.getpid()}.json"
    out_path.write_text(json.dumps(payload, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")
    # Keep benchmark index in sync for dashboard/report consumers.
    refresh_benchmark_index(out_dir.parent)
    return out_path
