"""Private skills benchmark snapshot helpers for benchmark tests."""

from __future__ import annotations

from dataclasses import dataclass
from datetime import UTC, datetime
from typing import Any

import yaml

from xiuxian_foundation.config.prj import get_skills_dir

DEFAULT_SNAPSHOT_SCHEMA = "omni.skills.tools_benchmark_snapshot.v1"
DEFAULT_REGRESSION_FACTOR = 2.0
DEFAULT_MIN_REGRESSION_DELTA_MS = 40.0


@dataclass(frozen=True, slots=True)
class SkillsSnapshotAnomaly:
    """One observed regression against snapshot baseline."""

    tool: str
    baseline_ms: float
    observed_ms: float
    threshold_ms: float
    regression_factor: float
    min_regression_delta_ms: float
    observed_metric: str = "avg_ms"
    scenario: str | None = None

    @property
    def delta_ms(self) -> float:
        return float(self.observed_ms - self.baseline_ms)

    @property
    def ratio(self) -> float:
        if self.baseline_ms <= 0:
            return 0.0
        return float(self.observed_ms / self.baseline_ms)

    def to_record(self) -> dict[str, Any]:
        return {
            "tool": self.tool,
            "baseline_ms": round(self.baseline_ms, 3),
            "observed_ms": round(self.observed_ms, 3),
            "observed_metric": self.observed_metric,
            "scenario": self.scenario,
            "threshold_ms": round(self.threshold_ms, 3),
            "delta_ms": round(self.delta_ms, 3),
            "ratio": round(self.ratio, 3),
            "regression_factor": round(self.regression_factor, 3),
            "min_regression_delta_ms": round(self.min_regression_delta_ms, 3),
        }


def default_skills_snapshot_path():
    return get_skills_dir() / "_snapshots" / "benchmark" / "skills_tools.yaml"


def load_skills_snapshot(path):
    if not path.exists():
        return None
    raw = yaml.safe_load(path.read_text(encoding="utf-8"))
    if not isinstance(raw, dict):
        return None
    return raw


def save_skills_snapshot(path, payload: dict[str, Any]):
    path.parent.mkdir(parents=True, exist_ok=True)
    text = yaml.safe_dump(payload, sort_keys=False, allow_unicode=False)
    path.write_text(text, encoding="utf-8")
    return path


def _as_float(value: Any) -> float | None:
    if isinstance(value, bool):
        return None
    if isinstance(value, int | float):
        return float(value)
    return None


def _positive_float(value: Any, fallback: float) -> float:
    parsed = _as_float(value)
    if parsed is None or parsed <= 0:
        return float(fallback)
    return float(parsed)


def build_skills_snapshot_payload(
    *,
    results: list[dict[str, Any]],
    runs_per_tool: int,
    warm_phase: bool,
    previous: dict[str, Any] | None = None,
    alpha: float = 0.35,
    default_regression_factor: float = DEFAULT_REGRESSION_FACTOR,
    default_min_regression_delta_ms: float = DEFAULT_MIN_REGRESSION_DELTA_MS,
) -> dict[str, Any]:
    clamped_alpha = max(0.0, min(1.0, float(alpha)))
    previous_tools = previous.get("tools") if isinstance(previous, dict) else None
    if not isinstance(previous_tools, dict):
        previous_tools = {}

    tools_out: dict[str, dict[str, Any]] = {}
    for tool_name, tool_payload in previous_tools.items():
        if isinstance(tool_name, str) and isinstance(tool_payload, dict):
            tools_out[tool_name] = dict(tool_payload)

    for result in results:
        tool_name = str(result.get("tool") or "").strip()
        if not tool_name:
            continue
        avg_ms = _as_float(result.get("avg_ms"))
        if avg_ms is None or avg_ms < 0:
            continue
        p50_ms = _as_float(result.get("p50_ms"))
        if p50_ms is None or p50_ms < 0:
            p50_ms = float(avg_ms)
        p95_ms = _as_float(result.get("p95_ms"))
        if p95_ms is None or p95_ms < 0:
            p95_ms = float(avg_ms)
        min_ms = _as_float(result.get("min_ms"))
        max_ms = _as_float(result.get("max_ms"))
        stdev_ms = _as_float(result.get("stdev_ms"))

        prior_entry = previous_tools.get(tool_name)
        prior_baseline = None
        if isinstance(prior_entry, dict):
            prior_baseline = _as_float(prior_entry.get("baseline_ms"))

        if prior_baseline is None:
            baseline_ms = float(p50_ms)
        else:
            baseline_ms = (prior_baseline * (1.0 - clamped_alpha)) + (float(p50_ms) * clamped_alpha)

        new_entry: dict[str, Any] = {
            "baseline_ms": round(baseline_ms, 3),
            "last_avg_ms": round(float(avg_ms), 3),
            "last_p50_ms": round(float(p50_ms), 3),
            "last_p95_ms": round(float(p95_ms), 3),
            "runs": int(result.get("runs", 0) or 0),
            "ok": bool(result.get("ok", False)),
        }
        scenario = result.get("scenario")
        if isinstance(scenario, str) and scenario:
            new_entry["scenario"] = scenario
        if min_ms is not None and min_ms >= 0:
            new_entry["last_min_ms"] = round(float(min_ms), 3)
        if max_ms is not None and max_ms >= 0:
            new_entry["last_max_ms"] = round(float(max_ms), 3)
        if stdev_ms is not None and stdev_ms >= 0:
            new_entry["last_stdev_ms"] = round(float(stdev_ms), 3)

        if isinstance(prior_entry, dict):
            for key in ("regression_factor", "min_regression_delta_ms"):
                if key in prior_entry:
                    new_entry[key] = prior_entry[key]

        tools_out[tool_name] = new_entry

    return {
        "schema": DEFAULT_SNAPSHOT_SCHEMA,
        "updated_at_utc": datetime.now(UTC).isoformat(),
        "benchmark": {
            "runs_per_tool": int(runs_per_tool),
            "warm_phase": bool(warm_phase),
        },
        "defaults": {
            "regression_factor": float(default_regression_factor),
            "min_regression_delta_ms": float(default_min_regression_delta_ms),
        },
        "tools": dict(sorted(tools_out.items(), key=lambda item: item[0])),
    }


def detect_skills_snapshot_anomalies(
    *,
    results: list[dict[str, Any]],
    snapshot: dict[str, Any] | None,
    default_regression_factor: float = DEFAULT_REGRESSION_FACTOR,
    default_min_regression_delta_ms: float = DEFAULT_MIN_REGRESSION_DELTA_MS,
) -> list[SkillsSnapshotAnomaly]:
    if not isinstance(snapshot, dict):
        return []

    snapshot_tools = snapshot.get("tools")
    if not isinstance(snapshot_tools, dict):
        return []

    defaults = snapshot.get("defaults")
    defaults_obj = defaults if isinstance(defaults, dict) else {}
    global_factor = _positive_float(
        defaults_obj.get("regression_factor"),
        fallback=default_regression_factor,
    )
    global_delta = _positive_float(
        defaults_obj.get("min_regression_delta_ms"),
        fallback=default_min_regression_delta_ms,
    )

    anomalies: list[SkillsSnapshotAnomaly] = []
    for result in results:
        tool_name = str(result.get("tool") or "").strip()
        if not tool_name:
            continue
        if not bool(result.get("ok", False)):
            continue
        override_metric_name = str(result.get("anomaly_observed_metric") or "").strip()
        override_observed_ms = _as_float(result.get("anomaly_observed_ms"))
        observed_avg = _as_float(result.get("avg_ms"))
        observed_p95 = _as_float(result.get("p95_ms"))
        runs = int(result.get("runs", 0) or 0)
        observed_metric = "avg_ms"
        observed_ms = observed_avg
        scenario = result.get("scenario")
        if not isinstance(scenario, str) or not scenario:
            scenario = None
        if override_observed_ms is not None and override_observed_ms >= 0:
            observed_ms = override_observed_ms
            observed_metric = override_metric_name or "custom"
        elif observed_p95 is not None and observed_p95 >= 0 and runs >= 5:
            observed_ms = observed_p95
            observed_metric = "p95_ms"
        if observed_ms is None or observed_ms < 0:
            continue

        tracked = snapshot_tools.get(tool_name)
        if not isinstance(tracked, dict):
            continue
        baseline_ms = _as_float(tracked.get("baseline_ms"))
        if baseline_ms is None or baseline_ms <= 0:
            continue

        factor = _positive_float(tracked.get("regression_factor"), fallback=global_factor)
        delta = _positive_float(tracked.get("min_regression_delta_ms"), fallback=global_delta)
        threshold_ms = max(float(baseline_ms) * factor, float(baseline_ms) + delta)

        if float(observed_ms) > threshold_ms:
            anomalies.append(
                SkillsSnapshotAnomaly(
                    tool=tool_name,
                    baseline_ms=float(baseline_ms),
                    observed_ms=float(observed_ms),
                    threshold_ms=float(threshold_ms),
                    regression_factor=float(factor),
                    min_regression_delta_ms=float(delta),
                    observed_metric=observed_metric,
                    scenario=scenario,
                )
            )

    return anomalies
