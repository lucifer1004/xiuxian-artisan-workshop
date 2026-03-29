#!/usr/bin/env python3
"""
Validate xiuxian-wendao PPR gate JSON report artifacts.

This script checks both:
- retrieval_eval.json
- related_benchmark.json
for the primary gate report directory, and optionally for mixed-scope canary reports.
"""

from __future__ import annotations

import argparse
import json
import os
from pathlib import Path
from typing import Any

from xiuxian_wendao_py.compat.runtime import get_project_root

RETRIEVAL_SCHEMA = "xiuxian_wendao.retrieval_eval.v1"
RELATED_SCHEMA = "xiuxian_wendao.related_benchmark.v1"


def _resolve_project_root() -> Path:
    try:
        return get_project_root().resolve()
    except Exception:
        prj_root = os.environ.get("PRJ_ROOT")
        if prj_root:
            return Path(prj_root).expanduser().resolve()
        return Path(".").resolve()


def _resolve_dir(project_root: Path, raw: str) -> Path:
    value = Path(raw).expanduser()
    if value.is_absolute():
        return value
    return (project_root / value).resolve()


def _is_number(value: Any) -> bool:
    return isinstance(value, (int, float)) and not isinstance(value, bool)


def _validate_retrieval_payload(payload: Any) -> list[str]:
    errors: list[str] = []
    if not isinstance(payload, dict):
        return ["retrieval report must be a JSON object"]

    summary = payload.get("summary")
    if not isinstance(summary, dict):
        errors.append("retrieval.summary must be an object")
        return errors

    schema = summary.get("schema")
    if schema != RETRIEVAL_SCHEMA:
        errors.append(
            f"retrieval.summary.schema mismatch: expected {RETRIEVAL_SCHEMA}, got {schema!r}"
        )

    required_int_keys = [
        "total_cases",
        "top1_count",
        "top3_count",
        "top10_count",
        "error_count",
    ]
    for key in required_int_keys:
        value = summary.get(key)
        if not isinstance(value, int):
            errors.append(f"retrieval.summary.{key} must be int")

    required_rate_keys = ["top1_rate", "top3_rate", "top10_rate"]
    for key in required_rate_keys:
        value = summary.get(key)
        if not _is_number(value):
            errors.append(f"retrieval.summary.{key} must be numeric")
        elif not (0.0 <= float(value) <= 1.0):
            errors.append(f"retrieval.summary.{key} must be within [0, 1]")

    cases = payload.get("cases")
    if not isinstance(cases, list):
        errors.append("retrieval.cases must be a list")
    failed_cases = payload.get("failed_cases")
    if not isinstance(failed_cases, list):
        errors.append("retrieval.failed_cases must be a list")

    return errors


def _validate_related_payload(payload: Any) -> list[str]:
    errors: list[str] = []
    if not isinstance(payload, dict):
        return ["related benchmark report must be a JSON object"]

    schema = payload.get("schema")
    if schema != RELATED_SCHEMA:
        errors.append(f"related.schema mismatch: expected {RELATED_SCHEMA}, got {schema!r}")

    summary = payload.get("summary")
    if not isinstance(summary, dict):
        errors.append("related.summary must be an object")
    else:
        for key in [
            "avg_ms",
            "median_ms",
            "p95_ms",
            "min_ms",
            "max_ms",
            "avg_subgraph_count",
            "avg_kernel_duration_ms",
            "avg_partition_duration_ms",
            "avg_fusion_duration_ms",
            "avg_total_duration_ms",
        ]:
            if not _is_number(summary.get(key)):
                errors.append(f"related.summary.{key} must be numeric")

    thresholds = payload.get("thresholds")
    if not isinstance(thresholds, dict):
        errors.append("related.thresholds must be an object")
    else:
        if not _is_number(thresholds.get("max_p95_ms")):
            errors.append("related.thresholds.max_p95_ms must be numeric")
        if not _is_number(thresholds.get("max_avg_ms")):
            errors.append("related.thresholds.max_avg_ms must be numeric")
        if not isinstance(thresholds.get("expect_subgraph_count_min"), int):
            errors.append("related.thresholds.expect_subgraph_count_min must be int")

    if not isinstance(payload.get("gates_failed"), list):
        errors.append("related.gates_failed must be a list")
    if not isinstance(payload.get("runs_detail"), list):
        errors.append("related.runs_detail must be a list")

    profile = payload.get("profile")
    if profile not in {"debug", "release"}:
        errors.append("related.profile must be 'debug' or 'release'")

    return errors


def _load_json(path: Path) -> tuple[Any | None, str | None]:
    if not path.exists():
        return None, f"missing file: {path}"
    try:
        return json.loads(path.read_text(encoding="utf-8")), None
    except Exception as exc:
        return None, f"invalid JSON at {path}: {exc}"


def _validate_report_pair(report_dir: Path) -> tuple[list[str], list[str]]:
    errors: list[str] = []
    warnings: list[str] = []

    retrieval_path = report_dir / "retrieval_eval.json"
    related_path = report_dir / "related_benchmark.json"

    retrieval_payload, retrieval_err = _load_json(retrieval_path)
    if retrieval_err:
        errors.append(retrieval_err)
    else:
        errors.extend(_validate_retrieval_payload(retrieval_payload))

    related_payload, related_err = _load_json(related_path)
    if related_err:
        errors.append(related_err)
    else:
        errors.extend(_validate_related_payload(related_payload))

    return errors, warnings


def _validate_optional_mixed_report_pair(report_dir: Path) -> tuple[list[str], list[str]]:
    errors: list[str] = []
    warnings: list[str] = []

    retrieval_path = report_dir / "retrieval_eval.json"
    related_path = report_dir / "related_benchmark.json"
    retrieval_exists = retrieval_path.exists()
    related_exists = related_path.exists()

    if not report_dir.exists() or (not retrieval_exists and not related_exists):
        warnings.append(f"mixed canary reports not found, skipped: {report_dir}")
        return errors, warnings

    if retrieval_exists != related_exists:
        errors.append(
            "mixed canary reports are incomplete: "
            f"retrieval_eval.json exists={retrieval_exists}, related_benchmark.json exists={related_exists}"
        )
        return errors, warnings

    pair_errors, pair_warnings = _validate_report_pair(report_dir)
    errors.extend(pair_errors)
    warnings.extend(pair_warnings)
    return errors, warnings


def main() -> int:
    parser = argparse.ArgumentParser(
        description="Validate xiuxian-wendao gate JSON report artifacts"
    )
    parser.add_argument("--root", default=".", help="project root")
    parser.add_argument(
        "--report-dir",
        default=os.environ.get("XIUXIAN_WENDAO_GATE_REPORT_DIR", ".run/reports/wendao-ppr-gate"),
        help="base gate report directory",
    )
    parser.add_argument(
        "--mixed-report-dir",
        default=os.environ.get(
            "XIUXIAN_WENDAO_MIXED_CANARY_REPORT_DIR",
            ".run/reports/wendao-ppr-mixed-canary",
        ),
        help="mixed-scope canary report directory",
    )
    parser.add_argument(
        "--require-mixed",
        action="store_true",
        help="fail if mixed canary reports are missing",
    )
    parser.add_argument("--json", action="store_true", help="print JSON output")
    args = parser.parse_args()

    project_root = _resolve_project_root()
    root = _resolve_dir(project_root, str(args.root))
    report_dir = _resolve_dir(root, str(args.report_dir))
    mixed_report_dir = _resolve_dir(root, str(args.mixed_report_dir))

    base_errors, base_warnings = _validate_report_pair(report_dir)
    mixed_errors: list[str] = []
    mixed_warnings: list[str] = []
    if args.require_mixed:
        if not mixed_report_dir.exists():
            mixed_errors.append(f"mixed canary report directory missing: {mixed_report_dir}")
        else:
            pair_errors, pair_warnings = _validate_report_pair(mixed_report_dir)
            mixed_errors.extend(pair_errors)
            mixed_warnings.extend(pair_warnings)
    else:
        pair_errors, pair_warnings = _validate_optional_mixed_report_pair(mixed_report_dir)
        mixed_errors.extend(pair_errors)
        mixed_warnings.extend(pair_warnings)

    errors = base_errors + mixed_errors
    warnings = base_warnings + mixed_warnings

    payload = {
        "schema": "xiuxian_wendao.gate_reports.validation.v1",
        "ok": not errors,
        "report_dir": str(report_dir),
        "mixed_report_dir": str(mixed_report_dir),
        "errors": errors,
        "warnings": warnings,
    }

    if args.json:
        print(json.dumps(payload, ensure_ascii=True, indent=2))
    else:
        if errors:
            print("wendao gate report validation: FAIL")
            for item in errors:
                print(f"- {item}")
        else:
            print("wendao gate report validation: PASS")
        for item in warnings:
            print(f"[warn] {item}")

    return 1 if errors else 0


if __name__ == "__main__":
    raise SystemExit(main())
