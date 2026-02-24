"""Knowledge benchmark snapshot helpers (YAML).

This module stores a stable latency baseline for knowledge tools under ``SKILLS_DIR`` and
detects unusually large regressions with tolerant thresholds.
"""

from __future__ import annotations

from dataclasses import dataclass
from datetime import UTC, datetime
from typing import TYPE_CHECKING, Any

import yaml

from omni.foundation.config.skills import SKILLS_DIR

if TYPE_CHECKING:
    from pathlib import Path

DEFAULT_SNAPSHOT_SCHEMA = "omni.skills.knowledge_benchmark_snapshot.v1"
DEFAULT_REGRESSION_FACTOR = 2.0
DEFAULT_MIN_REGRESSION_DELTA_MS = 40.0


@dataclass(frozen=True, slots=True)
class KnowledgeSnapshotAnomaly:
    """One observed regression against snapshot baseline."""

    tool: str
    baseline_ms: float
    observed_ms: float
    threshold_ms: float
    regression_factor: float
    min_regression_delta_ms: float

    @property
    def delta_ms(self) -> float:
        return float(self.observed_ms - self.baseline_ms)

    @property
    def ratio(self) -> float:
        if self.baseline_ms <= 0:
            return 0.0
        return float(self.observed_ms / self.baseline_ms)

    def to_record(self) -> dict[str, Any]:
        """Serialize anomaly to JSON/YAML-friendly record."""
        return {
            "tool": self.tool,
            "baseline_ms": round(self.baseline_ms, 3),
            "observed_ms": round(self.observed_ms, 3),
            "threshold_ms": round(self.threshold_ms, 3),
            "delta_ms": round(self.delta_ms, 3),
            "ratio": round(self.ratio, 3),
            "regression_factor": round(self.regression_factor, 3),
            "min_regression_delta_ms": round(self.min_regression_delta_ms, 3),
        }


def default_knowledge_snapshot_path() -> Path:
    """Return default snapshot path under ``SKILLS_DIR``."""
    return SKILLS_DIR() / "_snapshots" / "benchmark" / "knowledge_tools.yaml"


def load_knowledge_snapshot(path: Path) -> dict[str, Any] | None:
    """Load YAML snapshot file if present and valid."""
    if not path.exists():
        return None
    raw = yaml.safe_load(path.read_text(encoding="utf-8"))
    if not isinstance(raw, dict):
        return None
    return raw


def save_knowledge_snapshot(path: Path, payload: dict[str, Any]) -> Path:
    """Persist snapshot payload as YAML."""
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


def build_knowledge_snapshot_payload(
    *,
    results: list[dict[str, Any]],
    runs_per_tool: int,
    warm_phase: bool,
    previous: dict[str, Any] | None = None,
    alpha: float = 0.35,
    default_regression_factor: float = DEFAULT_REGRESSION_FACTOR,
    default_min_regression_delta_ms: float = DEFAULT_MIN_REGRESSION_DELTA_MS,
) -> dict[str, Any]:
    """Build YAML snapshot payload from benchmark results.

    ``alpha`` controls baseline smoothing when a previous snapshot exists:
    - ``alpha=1.0``: replace baseline with current value
    - ``alpha=0.0``: keep previous baseline
    """
    clamped_alpha = max(0.0, min(1.0, float(alpha)))
    previous_tools = previous.get("tools") if isinstance(previous, dict) else None
    if not isinstance(previous_tools, dict):
        previous_tools = {}

    tools_out: dict[str, dict[str, Any]] = {}
    # Keep existing entries unless current run updates them.
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

        prior_entry = previous_tools.get(tool_name)
        prior_baseline = None
        if isinstance(prior_entry, dict):
            prior_baseline = _as_float(prior_entry.get("baseline_ms"))

        if prior_baseline is None:
            baseline_ms = float(avg_ms)
        else:
            baseline_ms = (prior_baseline * (1.0 - clamped_alpha)) + (float(avg_ms) * clamped_alpha)

        new_entry: dict[str, Any] = {
            "baseline_ms": round(baseline_ms, 3),
            "last_avg_ms": round(float(avg_ms), 3),
            "runs": int(result.get("runs", 0) or 0),
            "ok": bool(result.get("ok", False)),
        }

        # Preserve optional per-tool threshold overrides.
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


def detect_knowledge_snapshot_anomalies(
    *,
    results: list[dict[str, Any]],
    snapshot: dict[str, Any] | None,
    default_regression_factor: float = DEFAULT_REGRESSION_FACTOR,
    default_min_regression_delta_ms: float = DEFAULT_MIN_REGRESSION_DELTA_MS,
) -> list[KnowledgeSnapshotAnomaly]:
    """Detect large latency spikes relative to snapshot baselines."""
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

    anomalies: list[KnowledgeSnapshotAnomaly] = []
    for result in results:
        tool_name = str(result.get("tool") or "").strip()
        if not tool_name:
            continue
        if not bool(result.get("ok", False)):
            continue
        observed_ms = _as_float(result.get("avg_ms"))
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
                KnowledgeSnapshotAnomaly(
                    tool=tool_name,
                    baseline_ms=float(baseline_ms),
                    observed_ms=float(observed_ms),
                    threshold_ms=float(threshold_ms),
                    regression_factor=float(factor),
                    min_regression_delta_ms=float(delta),
                )
            )

    return anomalies


__all__ = [
    "DEFAULT_MIN_REGRESSION_DELTA_MS",
    "DEFAULT_REGRESSION_FACTOR",
    "DEFAULT_SNAPSHOT_SCHEMA",
    "KnowledgeSnapshotAnomaly",
    "build_knowledge_snapshot_payload",
    "default_knowledge_snapshot_path",
    "detect_knowledge_snapshot_anomalies",
    "load_knowledge_snapshot",
    "save_knowledge_snapshot",
]
