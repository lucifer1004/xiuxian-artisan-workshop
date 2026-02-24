#!/usr/bin/env python3
"""Extract compact rollout gate line from wendao rollout status JSON."""

from __future__ import annotations

import argparse
import json
from pathlib import Path
from typing import Any


def _as_int(value: Any, default: int = 0) -> int:
    if isinstance(value, bool):
        return default
    if isinstance(value, int):
        return value
    if isinstance(value, float):
        return int(value)
    if isinstance(value, str) and value.strip():
        try:
            return int(value.strip())
        except ValueError:
            return default
    return default


def _build_fallback_line(payload: dict[str, Any], ready: bool) -> str:
    readiness = payload.get("readiness", {})
    streaks = payload.get("streaks", {})
    criteria = payload.get("criteria", {})
    streak = _as_int(streaks.get("both_ok"))
    required = _as_int(criteria.get("required_consecutive_runs"))
    remaining = _as_int(readiness.get("remaining_consecutive_runs"))
    blockers = readiness.get("blockers", [])
    blockers_text = (
        "|".join(str(item) for item in blockers)
        if isinstance(blockers, list) and blockers
        else "none"
    )
    return (
        "WENDAO_ROLLOUT "
        f"ready={str(ready).lower()} "
        f"streak={streak}/{required} "
        f"remaining={remaining} "
        f"blockers={blockers_text}"
    )


def render_rollout_gate_line(*, status_path: Path) -> tuple[str, str]:
    payload = json.loads(status_path.read_text(encoding="utf-8"))
    if not isinstance(payload, dict):
        raise ValueError("status payload must be a JSON object")

    readiness = payload.get("readiness", {})
    ready = bool(readiness.get("ready", False))
    ready_flag = "1" if ready else "0"
    line = str(readiness.get("gate_log_line", "")).strip()
    if not line:
        line = _build_fallback_line(payload, ready)
    return ready_flag, line


def main() -> int:
    parser = argparse.ArgumentParser(description="Extract compact WENDAO_ROLLOUT line")
    parser.add_argument("--status-json", required=True, help="wendao_rollout_status.json path")
    args = parser.parse_args()

    status_path = Path(str(args.status_json)).expanduser().resolve()
    ready_flag, line = render_rollout_gate_line(status_path=status_path)
    print(f"{ready_flag}\t{line}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
