#!/usr/bin/env python3
"""Render xiuxian-wendao PPR rollout readiness status from gate report artifacts."""

from __future__ import annotations

import argparse
import json
from pathlib import Path
from typing import Any

STATUS_SCHEMA = "xiuxian_wendao.rollout_status.v1"
RETRIEVAL_SCHEMA = "xiuxian_wendao.retrieval_eval.v1"
RELATED_SCHEMA = "xiuxian_wendao.related_benchmark.v1"
VALIDATION_SCHEMA = "xiuxian_wendao.gate_reports.validation.v1"


def _load_json(path: Path) -> tuple[dict[str, Any] | None, str | None]:
    if not path.exists():
        return None, "missing"
    try:
        payload = json.loads(path.read_text(encoding="utf-8"))
    except Exception as exc:
        return None, f"parse_error: {exc}"
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


def _bounded_ratio(value: Any) -> float:
    return max(0.0, min(1.0, _as_float(value)))


def _parse_retrieval_summary(path: Path) -> tuple[dict[str, Any], list[str]]:
    payload, error = _load_json(path)
    if error is not None:
        return {}, [f"{path}: {error}"]
    summary = payload.get("summary")
    if not isinstance(summary, dict):
        return {}, [f"{path}: summary must be object"]
    schema = str(summary.get("schema", "")).strip()
    if schema != RETRIEVAL_SCHEMA:
        return {}, [f"{path}: summary.schema mismatch ({schema!r})"]
    return {
        "top3_rate": _bounded_ratio(summary.get("top3_rate")),
        "top1_rate": _bounded_ratio(summary.get("top1_rate")),
        "top10_rate": _bounded_ratio(summary.get("top10_rate")),
        "total_cases": _as_int(summary.get("total_cases")),
        "error_count": _as_int(summary.get("error_count")),
    }, []


def _parse_related_summary(path: Path) -> tuple[dict[str, Any], list[str]]:
    payload, error = _load_json(path)
    if error is not None:
        return {}, [f"{path}: {error}"]
    schema = str(payload.get("schema", "")).strip()
    if schema != RELATED_SCHEMA:
        return {}, [f"{path}: schema mismatch ({schema!r})"]
    summary = payload.get("summary")
    if not isinstance(summary, dict):
        return {}, [f"{path}: summary must be object"]
    gates_failed = payload.get("gates_failed")
    if not isinstance(gates_failed, list):
        return {}, [f"{path}: gates_failed must be list"]
    return {
        "avg_ms": _as_float(summary.get("avg_ms")),
        "p95_ms": _as_float(summary.get("p95_ms")),
        "failed_runs": _as_int(summary.get("failed_runs")),
        "gates_failed_count": len(gates_failed),
        "gates_failed": [str(item) for item in gates_failed],
    }, []


def _parse_validation(path: Path) -> tuple[dict[str, Any], list[str]]:
    payload, error = _load_json(path)
    if error is not None:
        return {}, [f"{path}: {error}"]
    schema = str(payload.get("schema", "")).strip()
    if schema != VALIDATION_SCHEMA:
        return {}, [f"{path}: schema mismatch ({schema!r})"]
    errors = payload.get("errors")
    warnings = payload.get("warnings")
    if not isinstance(errors, list) or not isinstance(warnings, list):
        return {}, [f"{path}: errors/warnings must be lists"]
    return {
        "ok": bool(payload.get("ok", False)),
        "errors_count": len(errors),
        "warnings_count": len(warnings),
    }, []


def _next_streak(previous: int, ok: bool) -> int:
    if ok:
        return max(0, previous) + 1
    return 0


def _build_markdown(payload: dict[str, Any]) -> str:
    readiness = payload.get("readiness", {})
    current = payload.get("current", {})
    streaks = payload.get("streaks", {})
    criteria = payload.get("criteria", {})
    base = current.get("base_gate", {})
    mixed = current.get("mixed_canary", {})
    validation = current.get("report_validation", {})

    lines = [
        "# Wendao Rollout Status",
        "",
        f"- Ready: `{str(readiness.get('ready', False)).lower()}`",
        f"- Recommended default scope switch to `mixed`: `{str(readiness.get('recommended_default_scope_switch', False)).lower()}`",
        f"- Required consecutive runs: `{criteria.get('required_consecutive_runs', 0)}`",
        f"- Current consecutive runs (`both_ok`): `{streaks.get('both_ok', 0)}`",
        f"- Remaining consecutive runs: `{readiness.get('remaining_consecutive_runs', 0)}`",
        f"- Mixed canary min Top3 rate: `{criteria.get('required_mixed_top3_rate', 0.0):.2f}`",
        f"- Blockers: `{', '.join(readiness.get('blockers', [])) if readiness.get('blockers') else 'none'}`",
        "",
        "## Current Gate Signals",
        "",
        f"- Base gate ok: `{str(base.get('ok', False)).lower()}` (top3_rate={base.get('top3_rate', 0.0):.4f}, related_gate_failures={base.get('related_gates_failed_count', 0)})",
        f"- Mixed canary ok: `{str(mixed.get('ok', False)).lower()}` (top3_rate={mixed.get('top3_rate', 0.0):.4f}, related_gate_failures={mixed.get('related_gates_failed_count', 0)})",
        f"- Report validation ok: `{str(validation.get('ok', False)).lower()}` (errors={validation.get('errors_count', 0)}, warnings={validation.get('warnings_count', 0)})",
        "",
        "## Streaks",
        "",
        f"- base_gate_ok: `{streaks.get('base_gate_ok', 0)}`",
        f"- mixed_canary_ok: `{streaks.get('mixed_canary_ok', 0)}`",
        f"- report_validation_ok: `{streaks.get('report_validation_ok', 0)}`",
        f"- both_ok: `{streaks.get('both_ok', 0)}`",
        "",
    ]
    return "\n".join(lines) + "\n"


def _build_gate_log_line(
    *,
    ready: bool,
    streak: int,
    required_runs: int,
    remaining_runs: int,
    blockers: list[str],
) -> str:
    blockers_text = "|".join(blockers) if blockers else "none"
    return (
        "WENDAO_ROLLOUT "
        f"ready={str(ready).lower()} "
        f"streak={streak}/{required_runs} "
        f"remaining={remaining_runs} "
        f"blockers={blockers_text}"
    )


def render_rollout_status(
    *,
    base_report_dir: Path,
    mixed_report_dir: Path,
    validation_report_path: Path,
    previous_status_path: Path | None,
    required_consecutive_runs: int,
    required_mixed_top3_rate: float,
) -> tuple[dict[str, Any], list[str]]:
    errors: list[str] = []

    base_retrieval, base_retrieval_errors = _parse_retrieval_summary(
        base_report_dir / "retrieval_eval.json"
    )
    base_related, base_related_errors = _parse_related_summary(
        base_report_dir / "related_benchmark.json"
    )
    mixed_retrieval, mixed_retrieval_errors = _parse_retrieval_summary(
        mixed_report_dir / "retrieval_eval.json"
    )
    mixed_related, mixed_related_errors = _parse_related_summary(
        mixed_report_dir / "related_benchmark.json"
    )
    validation, validation_errors = _parse_validation(validation_report_path)

    errors.extend(base_retrieval_errors)
    errors.extend(base_related_errors)
    errors.extend(mixed_retrieval_errors)
    errors.extend(mixed_related_errors)
    errors.extend(validation_errors)

    previous_streaks: dict[str, int] = {}
    if previous_status_path is not None:
        previous_payload, previous_error = _load_json(previous_status_path)
        if previous_error is None and isinstance(previous_payload, dict):
            raw_streaks = previous_payload.get("streaks")
            if isinstance(raw_streaks, dict):
                previous_streaks = {
                    key: _as_int(value)
                    for key, value in raw_streaks.items()
                    if isinstance(key, str)
                }

    base_ok = (
        not base_retrieval_errors
        and not base_related_errors
        and base_retrieval.get("error_count", 0) == 0
        and base_related.get("gates_failed_count", 1) == 0
    )
    mixed_ok = (
        not mixed_retrieval_errors
        and not mixed_related_errors
        and mixed_retrieval.get("error_count", 0) == 0
        and mixed_retrieval.get("top3_rate", 0.0) >= required_mixed_top3_rate
        and mixed_related.get("gates_failed_count", 1) == 0
    )
    validation_ok = not validation_errors and bool(validation.get("ok", False))
    both_ok = base_ok and mixed_ok and validation_ok

    streaks = {
        "base_gate_ok": _next_streak(previous_streaks.get("base_gate_ok", 0), base_ok),
        "mixed_canary_ok": _next_streak(previous_streaks.get("mixed_canary_ok", 0), mixed_ok),
        "report_validation_ok": _next_streak(
            previous_streaks.get("report_validation_ok", 0), validation_ok
        ),
        "both_ok": _next_streak(previous_streaks.get("both_ok", 0), both_ok),
    }

    readiness_ok = streaks["both_ok"] >= required_consecutive_runs
    remaining_runs = max(0, required_consecutive_runs - streaks["both_ok"])
    blockers: list[str] = []
    if errors:
        blockers.append("report_parse_or_schema_error")
    if not base_ok:
        blockers.append("base_gate_not_green")
    if not mixed_ok:
        blockers.append("mixed_canary_not_green")
    if not validation_ok:
        blockers.append("report_validation_failed")
    if remaining_runs > 0:
        blockers.append(f"consecutive_runs_remaining:{remaining_runs}")

    gate_log_line = _build_gate_log_line(
        ready=readiness_ok,
        streak=streaks["both_ok"],
        required_runs=required_consecutive_runs,
        remaining_runs=remaining_runs,
        blockers=blockers,
    )

    payload: dict[str, Any] = {
        "schema": STATUS_SCHEMA,
        "criteria": {
            "required_consecutive_runs": required_consecutive_runs,
            "required_mixed_top3_rate": required_mixed_top3_rate,
        },
        "current": {
            "base_gate": {
                "ok": base_ok,
                "top3_rate": _as_float(base_retrieval.get("top3_rate")),
                "related_gates_failed_count": _as_int(base_related.get("gates_failed_count")),
                "errors": base_retrieval_errors + base_related_errors,
            },
            "mixed_canary": {
                "ok": mixed_ok,
                "top3_rate": _as_float(mixed_retrieval.get("top3_rate")),
                "related_gates_failed_count": _as_int(mixed_related.get("gates_failed_count")),
                "errors": mixed_retrieval_errors + mixed_related_errors,
            },
            "report_validation": {
                "ok": validation_ok,
                "errors_count": _as_int(validation.get("errors_count")),
                "warnings_count": _as_int(validation.get("warnings_count")),
                "errors": validation_errors,
            },
        },
        "streaks": streaks,
        "readiness": {
            "ready": readiness_ok,
            "recommended_default_scope_switch": readiness_ok,
            "remaining_consecutive_runs": remaining_runs,
            "blockers": blockers,
            "gate_log_line": gate_log_line,
        },
        "inputs": {
            "base_report_dir": str(base_report_dir),
            "mixed_report_dir": str(mixed_report_dir),
            "validation_report": str(validation_report_path),
            "previous_status": str(previous_status_path) if previous_status_path else "",
        },
        "errors": errors,
    }
    return payload, errors


def main() -> int:
    parser = argparse.ArgumentParser(description="Render xiuxian-wendao rollout status")
    parser.add_argument(
        "--base-report-dir",
        default=".run/reports/wendao-ppr-gate",
        help="base gate report directory",
    )
    parser.add_argument(
        "--mixed-report-dir",
        default=".run/reports/wendao-ppr-mixed-canary",
        help="mixed canary report directory",
    )
    parser.add_argument(
        "--validation-report",
        default=".run/reports/wendao-ppr-gate/report_validation.json",
        help="validation report JSON path",
    )
    parser.add_argument(
        "--previous-status-json",
        default="",
        help="optional previous rollout status JSON path",
    )
    parser.add_argument(
        "--required-consecutive-runs",
        type=int,
        default=7,
        help="required consecutive both-ok runs before switch recommendation",
    )
    parser.add_argument(
        "--required-mixed-top3-rate",
        type=float,
        default=0.9,
        help="minimum mixed canary top3 rate",
    )
    parser.add_argument("--output-json", required=True, help="output rollout JSON path")
    parser.add_argument("--output-markdown", default="", help="optional markdown summary path")
    parser.add_argument(
        "--strict-ready",
        action="store_true",
        help="exit non-zero unless rollout is ready",
    )
    args = parser.parse_args()

    base_report_dir = Path(str(args.base_report_dir)).expanduser().resolve()
    mixed_report_dir = Path(str(args.mixed_report_dir)).expanduser().resolve()
    validation_report = Path(str(args.validation_report)).expanduser().resolve()
    previous_status = (
        Path(str(args.previous_status_json)).expanduser().resolve()
        if str(args.previous_status_json).strip()
        else None
    )
    output_json = Path(str(args.output_json)).expanduser().resolve()
    output_markdown = (
        Path(str(args.output_markdown)).expanduser().resolve()
        if str(args.output_markdown).strip()
        else None
    )

    payload, _errors = render_rollout_status(
        base_report_dir=base_report_dir,
        mixed_report_dir=mixed_report_dir,
        validation_report_path=validation_report,
        previous_status_path=previous_status,
        required_consecutive_runs=max(1, int(args.required_consecutive_runs)),
        required_mixed_top3_rate=max(0.0, min(1.0, float(args.required_mixed_top3_rate))),
    )

    output_json.parent.mkdir(parents=True, exist_ok=True)
    output_json.write_text(
        json.dumps(payload, ensure_ascii=True, indent=2) + "\n", encoding="utf-8"
    )
    if output_markdown is not None:
        output_markdown.parent.mkdir(parents=True, exist_ok=True)
        output_markdown.write_text(_build_markdown(payload), encoding="utf-8")

    ready = bool(payload.get("readiness", {}).get("ready", False))
    if args.strict_ready and not ready:
        return 1
    # Missing/parse errors remain encoded in payload; this command is advisory by default.
    return 0 if not args.strict_ready else (0 if ready else 1)


if __name__ == "__main__":
    raise SystemExit(main())
