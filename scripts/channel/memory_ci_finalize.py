#!/usr/bin/env python3

from __future__ import annotations

import argparse
import json
import re
import shutil
import time
from pathlib import Path


def newest_failure(
    reports_dir: Path,
    profile: str,
    *,
    extension: str,
    start_stamp: int,
) -> tuple[Path | None, int]:
    pattern = re.compile(rf"omni-agent-memory-ci-failure-{re.escape(profile)}-(\d+)\.{extension}$")
    best_path: Path | None = None
    best_stamp = -1
    for path in reports_dir.glob(f"omni-agent-memory-ci-failure-{profile}-*.{extension}"):
        match = pattern.match(path.name)
        if match is None:
            continue
        stamp = int(match.group(1))
        if stamp < start_stamp:
            continue
        if stamp > best_stamp:
            best_stamp = stamp
            best_path = path
    return best_path, best_stamp


def finalize_gate_run(
    *,
    reports_dir: Path,
    profile: str,
    start_stamp: int,
    exit_code: int,
    latest_failure_json: Path,
    latest_failure_md: Path,
    latest_run_json: Path,
    log_file: Path,
    finish_stamp: int,
) -> None:
    profile_title = profile.capitalize()

    reports_dir.mkdir(parents=True, exist_ok=True)
    latest_failure_json.parent.mkdir(parents=True, exist_ok=True)
    latest_failure_md.parent.mkdir(parents=True, exist_ok=True)
    latest_run_json.parent.mkdir(parents=True, exist_ok=True)

    picked_json_path, picked_json_stamp = newest_failure(
        reports_dir,
        profile,
        extension="json",
        start_stamp=start_stamp,
    )
    picked_md_path, picked_md_stamp = newest_failure(
        reports_dir,
        profile,
        extension="md",
        start_stamp=start_stamp,
    )

    if exit_code != 0:
        if picked_json_path is not None:
            shutil.copy2(picked_json_path, latest_failure_json)
        else:
            fallback_payload = {
                "generated_at_utc": time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime()),
                "profile": profile,
                "category": "runner_unknown_failure",
                "summary": f"{profile} gate failed before triage json emission",
                "error": f"exit_code={exit_code}",
                "artifacts": [
                    {
                        "name": "runtime_log",
                        "path": str(log_file),
                        "exists": bool(log_file.exists()),
                    }
                ],
                "repro_commands": [f"tail -n 200 {log_file}"],
            }
            latest_failure_json.write_text(
                json.dumps(fallback_payload, ensure_ascii=False, indent=2) + "\n",
                encoding="utf-8",
            )
        if picked_md_path is not None:
            shutil.copy2(picked_md_path, latest_failure_md)
        elif not latest_failure_md.exists():
            latest_failure_md.write_text(
                (
                    "# Omni Agent Memory CI Failure\n\n"
                    f"- profile: {profile}\n"
                    f"- exit_code: {exit_code}\n"
                    f"- log: {log_file}\n"
                ),
                encoding="utf-8",
            )

    status_payload = {
        "profile": profile,
        "started_at_ms": start_stamp,
        "finished_at_ms": finish_stamp,
        "duration_ms": max(0, finish_stamp - start_stamp),
        "exit_code": exit_code,
        "status": "passed" if exit_code == 0 else "failed",
        "log_file": str(log_file),
        "latest_failure_json": str(latest_failure_json) if latest_failure_json.exists() else "",
        "latest_failure_markdown": str(latest_failure_md) if latest_failure_md.exists() else "",
        "selected_failure_report_json": str(picked_json_path)
        if picked_json_path is not None
        else "",
        "selected_failure_report_json_stamp": picked_json_stamp if picked_json_stamp >= 0 else None,
        "selected_failure_report_markdown": str(picked_md_path)
        if picked_md_path is not None
        else "",
        "selected_failure_report_markdown_stamp": picked_md_stamp if picked_md_stamp >= 0 else None,
    }
    latest_run_json.write_text(
        json.dumps(status_payload, ensure_ascii=False, indent=2) + "\n",
        encoding="utf-8",
    )

    print(
        f"{profile_title} CI summary: "
        f"status={status_payload['status']} "
        f"exit_code={exit_code} "
        f"log={log_file} "
        f"latest_run={latest_run_json}"
    )
    if exit_code != 0:
        print(
            f"{profile_title} CI failure aggregates: "
            f"json={latest_failure_json} md={latest_failure_md}"
        )


def _parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description="Finalize memory CI gate run artifacts.")
    parser.add_argument("--reports-dir", required=True)
    parser.add_argument("--profile", required=True, choices=("quick", "nightly"))
    parser.add_argument("--start-stamp", required=True, type=int)
    parser.add_argument("--exit-code", required=True, type=int)
    parser.add_argument("--latest-failure-json", required=True)
    parser.add_argument("--latest-failure-md", required=True)
    parser.add_argument("--latest-run-json", required=True)
    parser.add_argument("--log-file", required=True)
    parser.add_argument("--finish-stamp", required=True, type=int)
    return parser.parse_args()


def main() -> int:
    args = _parse_args()
    finalize_gate_run(
        reports_dir=Path(args.reports_dir),
        profile=str(args.profile),
        start_stamp=int(args.start_stamp),
        exit_code=int(args.exit_code),
        latest_failure_json=Path(args.latest_failure_json),
        latest_failure_md=Path(args.latest_failure_md),
        latest_run_json=Path(args.latest_run_json),
        log_file=Path(args.log_file),
        finish_stamp=int(args.finish_stamp),
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
