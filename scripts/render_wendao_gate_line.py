#!/usr/bin/env python3
"""Render a compact WENDAO_PPR_GATE line from gate report artifacts."""

from __future__ import annotations

import argparse
import json
from pathlib import Path
from typing import Any

RETRIEVAL_SCHEMA = "xiuxian_wendao.retrieval_eval.v1"
RELATED_SCHEMA = "xiuxian_wendao.related_benchmark.v1"


def _load_json(path: Path) -> tuple[dict[str, Any] | None, str | None]:
    if not path.exists():
        return None, "missing"
    try:
        payload = json.loads(path.read_text(encoding="utf-8"))
    except Exception as exc:  # pragma: no cover - defensive shaping
        return None, f"invalid_json:{exc}"
    if not isinstance(payload, dict):
        return None, "invalid_payload"
    return payload, None


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


def _as_float(value: Any, default: float = 0.0) -> float:
    if isinstance(value, bool):
        return default
    if isinstance(value, (int, float)):
        return float(value)
    if isinstance(value, str) and value.strip():
        try:
            return float(value.strip())
        except ValueError:
            return default
    return default


def render_gate_line(
    *,
    retrieval_report: Path,
    related_report: Path,
    min_top3_rate: float,
    retrieval_exit_code: int,
    related_exit_code: int,
) -> str:
    retrieval_payload, retrieval_error = _load_json(retrieval_report)
    related_payload, related_error = _load_json(related_report)

    retrieval_summary = (
        retrieval_payload.get("summary", {}) if isinstance(retrieval_payload, dict) else {}
    )
    related_summary = (
        related_payload.get("summary", {}) if isinstance(related_payload, dict) else {}
    )

    retrieval_schema = str(retrieval_summary.get("schema", "")).strip()
    related_schema = str(related_payload.get("schema", "")).strip() if related_payload else ""
    top3_rate = _as_float(retrieval_summary.get("top3_rate"))
    retrieval_errors = _as_int(retrieval_summary.get("error_count"))
    p95_ms = _as_float(related_summary.get("p95_ms"))
    gates_failed = (
        related_payload.get("gates_failed", []) if isinstance(related_payload, dict) else []
    )
    gates_failed_count = len(gates_failed) if isinstance(gates_failed, list) else 0

    retrieval_ok = (
        retrieval_exit_code == 0
        and retrieval_error is None
        and retrieval_schema == RETRIEVAL_SCHEMA
        and retrieval_errors == 0
        and top3_rate >= min_top3_rate
    )
    related_ok = (
        related_exit_code == 0
        and related_error is None
        and related_schema == RELATED_SCHEMA
        and gates_failed_count == 0
    )

    blockers: list[str] = []
    if retrieval_error is not None:
        blockers.append(f"retrieval_report_{retrieval_error}")
    if related_error is not None:
        blockers.append(f"related_report_{related_error}")
    if retrieval_exit_code != 0:
        blockers.append(f"retrieval_gate_failed:rc={retrieval_exit_code}")
    if related_exit_code != 0:
        blockers.append(f"related_gate_failed:rc={related_exit_code}")
    blockers_text = "none" if not blockers else "|".join(blockers)

    return (
        "WENDAO_PPR_GATE "
        f"retrieval_ok={str(retrieval_ok).lower()} "
        f"top3_rate={top3_rate:.4f} "
        f"related_ok={str(related_ok).lower()} "
        f"related_p95_ms={p95_ms:.2f} "
        f"related_gates_failed={gates_failed_count} "
        f"blockers={blockers_text}"
    )


def main() -> int:
    parser = argparse.ArgumentParser(description="Render compact WENDAO_PPR_GATE line")
    parser.add_argument("--retrieval-report", required=True, help="retrieval_eval.json path")
    parser.add_argument("--related-report", required=True, help="related_benchmark.json path")
    parser.add_argument(
        "--min-top3-rate",
        type=float,
        default=1.0,
        help="minimum retrieval top3 threshold for retrieval_ok",
    )
    parser.add_argument(
        "--retrieval-exit-code",
        type=int,
        default=0,
        help="retrieval gate process exit code",
    )
    parser.add_argument(
        "--related-exit-code",
        type=int,
        default=0,
        help="related benchmark process exit code",
    )
    args = parser.parse_args()

    line = render_gate_line(
        retrieval_report=Path(str(args.retrieval_report)).expanduser().resolve(),
        related_report=Path(str(args.related_report)).expanduser().resolve(),
        min_top3_rate=max(0.0, min(1.0, float(args.min_top3_rate))),
        retrieval_exit_code=int(args.retrieval_exit_code),
        related_exit_code=int(args.related_exit_code),
    )
    print(line)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
