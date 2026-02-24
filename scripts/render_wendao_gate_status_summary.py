#!/usr/bin/env python3
"""Render xiuxian-wendao gate status summary from retrieval and related reports."""

from __future__ import annotations

import argparse
import json
from pathlib import Path
from typing import Any

STATUS_SCHEMA = "xiuxian_wendao.gate_status_summary.v1"
RETRIEVAL_SCHEMA = "xiuxian_wendao.retrieval_eval.v1"
RELATED_SCHEMA = "xiuxian_wendao.related_benchmark.v1"


def _load_json(path: Path) -> tuple[dict[str, Any] | None, str | None]:
    if not path.exists():
        return None, "missing"
    try:
        payload = json.loads(path.read_text(encoding="utf-8"))
    except Exception as exc:  # pragma: no cover - defensive error shaping
        return None, f"parse_error:{exc}"
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


def _build_gate_log_line(
    *,
    scope: str,
    retrieval_ok: bool,
    top3_rate: float,
    related_ok: bool,
    related_p95_ms: float,
    related_gates_failed: int,
    blockers: list[str],
) -> str:
    blockers_text = "|".join(blockers) if blockers else "none"
    return (
        "WENDAO_PPR_GATE "
        f"scope={scope} "
        f"retrieval_ok={str(retrieval_ok).lower()} "
        f"top3_rate={top3_rate:.4f} "
        f"related_ok={str(related_ok).lower()} "
        f"related_p95_ms={related_p95_ms:.2f} "
        f"related_gates_failed={related_gates_failed} "
        f"blockers={blockers_text}"
    )


def _build_scope_status(
    *,
    scope: str,
    report_dir: Path,
    min_top3_rate: float,
) -> tuple[dict[str, Any], list[str]]:
    errors: list[str] = []
    blockers: list[str] = []

    retrieval_payload, retrieval_error = _load_json(report_dir / "retrieval_eval.json")
    related_payload, related_error = _load_json(report_dir / "related_benchmark.json")

    retrieval_summary = (
        retrieval_payload.get("summary", {}) if isinstance(retrieval_payload, dict) else {}
    )
    retrieval_schema = str(retrieval_summary.get("schema", "")).strip()
    top3_rate = _as_float(retrieval_summary.get("top3_rate"))
    retrieval_error_count = _as_int(retrieval_summary.get("error_count"))
    retrieval_ok = (
        retrieval_error is None
        and retrieval_schema == RETRIEVAL_SCHEMA
        and retrieval_error_count == 0
        and top3_rate >= min_top3_rate
    )
    if retrieval_error is not None:
        reason = f"{scope}.retrieval:{retrieval_error}"
        errors.append(reason)
        blockers.append("retrieval_report_unavailable")
    elif retrieval_schema != RETRIEVAL_SCHEMA:
        reason = f"{scope}.retrieval:schema_mismatch:{retrieval_schema!r}"
        errors.append(reason)
        blockers.append("retrieval_schema_mismatch")
    if retrieval_error_count > 0:
        blockers.append(f"retrieval_error_count:{retrieval_error_count}")
    if top3_rate < min_top3_rate:
        blockers.append(f"top3_below_threshold:{top3_rate:.4f}<{min_top3_rate:.4f}")

    related_schema = str(related_payload.get("schema", "")).strip() if related_payload else ""
    related_summary = (
        related_payload.get("summary", {}) if isinstance(related_payload, dict) else {}
    )
    related_p95_ms = _as_float(related_summary.get("p95_ms"))
    gates_failed = (
        related_payload.get("gates_failed", []) if isinstance(related_payload, dict) else []
    )
    related_gates_failed = len(gates_failed) if isinstance(gates_failed, list) else 0
    related_ok = (
        related_error is None and related_schema == RELATED_SCHEMA and related_gates_failed == 0
    )
    if related_error is not None:
        reason = f"{scope}.related:{related_error}"
        errors.append(reason)
        blockers.append("related_report_unavailable")
    elif related_schema != RELATED_SCHEMA:
        reason = f"{scope}.related:schema_mismatch:{related_schema!r}"
        errors.append(reason)
        blockers.append("related_schema_mismatch")
    if related_gates_failed > 0:
        blockers.append(f"related_gates_failed:{related_gates_failed}")

    status = {
        "scope": scope,
        "report_dir": str(report_dir),
        "min_top3_rate": min_top3_rate,
        "ok": retrieval_ok and related_ok,
        "retrieval": {
            "ok": retrieval_ok,
            "top3_rate": top3_rate,
            "error_count": retrieval_error_count,
        },
        "related": {
            "ok": related_ok,
            "p95_ms": related_p95_ms,
            "gates_failed_count": related_gates_failed,
        },
        "blockers": blockers,
        "gate_log_line": _build_gate_log_line(
            scope=scope,
            retrieval_ok=retrieval_ok,
            top3_rate=top3_rate,
            related_ok=related_ok,
            related_p95_ms=related_p95_ms,
            related_gates_failed=related_gates_failed,
            blockers=blockers,
        ),
    }
    return status, errors


def _build_markdown(payload: dict[str, Any]) -> str:
    scopes = payload.get("scopes", {})
    base = scopes.get("base", {})
    mixed = scopes.get("mixed", {})
    overall = payload.get("overall", {})

    lines = [
        f"## Wendao Gate Summary ({payload.get('runner_os', 'unknown')})",
        "",
        f"- Overall gate healthy: `{str(overall.get('ok', False)).lower()}`",
        "",
        "| Scope | Ok | Top3 | Min Top3 | P95(ms) | Related Gate Failures | Blockers |",
        "| --- | --- | ---: | ---: | ---: | ---: | --- |",
        (
            "| base | "
            f"{str(base.get('ok', False)).lower()} | "
            f"{base.get('retrieval', {}).get('top3_rate', 0.0):.4f} | "
            f"{base.get('min_top3_rate', 0.0):.4f} | "
            f"{base.get('related', {}).get('p95_ms', 0.0):.2f} | "
            f"{base.get('related', {}).get('gates_failed_count', 0)} | "
            f"{' | '.join(base.get('blockers', [])) if base.get('blockers') else 'none'} |"
        ),
        (
            "| mixed | "
            f"{str(mixed.get('ok', False)).lower()} | "
            f"{mixed.get('retrieval', {}).get('top3_rate', 0.0):.4f} | "
            f"{mixed.get('min_top3_rate', 0.0):.4f} | "
            f"{mixed.get('related', {}).get('p95_ms', 0.0):.2f} | "
            f"{mixed.get('related', {}).get('gates_failed_count', 0)} | "
            f"{' | '.join(mixed.get('blockers', [])) if mixed.get('blockers') else 'none'} |"
        ),
        "",
        "- Gate lines:",
        f"  - `{base.get('gate_log_line', '')}`",
        f"  - `{mixed.get('gate_log_line', '')}`",
        "",
    ]
    return "\n".join(lines) + "\n"


def render_gate_status_summary(
    *,
    base_report_dir: Path,
    mixed_report_dir: Path,
    min_base_top3_rate: float,
    min_mixed_top3_rate: float,
    runner_os: str,
) -> tuple[dict[str, Any], list[str]]:
    base_status, base_errors = _build_scope_status(
        scope="base",
        report_dir=base_report_dir,
        min_top3_rate=min_base_top3_rate,
    )
    mixed_status, mixed_errors = _build_scope_status(
        scope="mixed",
        report_dir=mixed_report_dir,
        min_top3_rate=min_mixed_top3_rate,
    )
    errors = base_errors + mixed_errors
    payload: dict[str, Any] = {
        "schema": STATUS_SCHEMA,
        "runner_os": runner_os,
        "criteria": {
            "min_base_top3_rate": min_base_top3_rate,
            "min_mixed_top3_rate": min_mixed_top3_rate,
        },
        "scopes": {
            "base": base_status,
            "mixed": mixed_status,
        },
        "overall": {
            "ok": bool(base_status.get("ok", False)) and bool(mixed_status.get("ok", False)),
        },
        "log_lines": [
            str(base_status.get("gate_log_line", "")),
            str(mixed_status.get("gate_log_line", "")),
        ],
        "errors": errors,
    }
    return payload, errors


def main() -> int:
    parser = argparse.ArgumentParser(description="Render xiuxian-wendao gate status summary")
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
        "--min-base-top3-rate",
        type=float,
        default=1.0,
        help="minimum top3 rate required for base gate",
    )
    parser.add_argument(
        "--min-mixed-top3-rate",
        type=float,
        default=0.9,
        help="minimum top3 rate required for mixed canary",
    )
    parser.add_argument("--runner-os", default="", help="runner os label for summary")
    parser.add_argument("--output-json", default="", help="optional output JSON path")
    parser.add_argument("--output-markdown", default="", help="optional output markdown path")
    parser.add_argument(
        "--strict-green",
        action="store_true",
        help="exit non-zero unless both base and mixed are green",
    )
    args = parser.parse_args()

    base_report_dir = Path(str(args.base_report_dir)).expanduser().resolve()
    mixed_report_dir = Path(str(args.mixed_report_dir)).expanduser().resolve()
    output_json = (
        Path(str(args.output_json)).expanduser().resolve()
        if str(args.output_json).strip()
        else None
    )
    output_markdown = (
        Path(str(args.output_markdown)).expanduser().resolve()
        if str(args.output_markdown).strip()
        else None
    )

    payload, _errors = render_gate_status_summary(
        base_report_dir=base_report_dir,
        mixed_report_dir=mixed_report_dir,
        min_base_top3_rate=max(0.0, min(1.0, float(args.min_base_top3_rate))),
        min_mixed_top3_rate=max(0.0, min(1.0, float(args.min_mixed_top3_rate))),
        runner_os=str(args.runner_os).strip() or "local",
    )

    for line in payload.get("log_lines", []):
        if line:
            print(line)

    if output_json is not None:
        output_json.parent.mkdir(parents=True, exist_ok=True)
        output_json.write_text(
            json.dumps(payload, ensure_ascii=True, indent=2) + "\n", encoding="utf-8"
        )
    if output_markdown is not None:
        output_markdown.parent.mkdir(parents=True, exist_ok=True)
        output_markdown.write_text(_build_markdown(payload), encoding="utf-8")

    if args.strict_green and not bool(payload.get("overall", {}).get("ok", False)):
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
