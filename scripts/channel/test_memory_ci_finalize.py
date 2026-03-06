#!/usr/bin/env python3

from __future__ import annotations

import importlib.util
import json
from pathlib import Path

PROJECT_ROOT = Path(__file__).resolve().parents[2]
MODULE_PATH = PROJECT_ROOT / "scripts" / "channel" / "memory_ci_finalize.py"


def _load_module():
    spec = importlib.util.spec_from_file_location("memory_ci_finalize", MODULE_PATH)
    if spec is None or spec.loader is None:
        raise RuntimeError("failed to load memory_ci_finalize module")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def test_finalize_gate_run_success_writes_latest_run_only(tmp_path: Path) -> None:
    module = _load_module()

    reports_dir = tmp_path / "reports"
    latest_failure_json = tmp_path / "latest-failure.json"
    latest_failure_md = tmp_path / "latest-failure.md"
    latest_run_json = tmp_path / "latest-run.json"
    log_file = tmp_path / "runner.log"
    log_file.write_text("ok\n", encoding="utf-8")

    module.finalize_gate_run(
        reports_dir=reports_dir,
        profile="quick",
        start_stamp=100,
        exit_code=0,
        latest_failure_json=latest_failure_json,
        latest_failure_md=latest_failure_md,
        latest_run_json=latest_run_json,
        log_file=log_file,
        finish_stamp=120,
    )

    assert latest_run_json.exists()
    run_payload = json.loads(latest_run_json.read_text(encoding="utf-8"))
    assert run_payload["status"] == "passed"
    assert run_payload["exit_code"] == 0
    assert run_payload["duration_ms"] == 20
    assert run_payload["latest_failure_json"] == ""
    assert run_payload["latest_failure_markdown"] == ""
    assert run_payload["selected_failure_report_json"] == ""
    assert run_payload["selected_failure_report_markdown"] == ""
    assert not latest_failure_json.exists()
    assert not latest_failure_md.exists()


def test_finalize_gate_run_failure_without_report_uses_fallback_payload(tmp_path: Path) -> None:
    module = _load_module()

    reports_dir = tmp_path / "reports"
    latest_failure_json = tmp_path / "latest-failure.json"
    latest_failure_md = tmp_path / "latest-failure.md"
    latest_run_json = tmp_path / "latest-run.json"
    log_file = tmp_path / "runner.log"
    log_file.write_text("failed\n", encoding="utf-8")

    module.finalize_gate_run(
        reports_dir=reports_dir,
        profile="nightly",
        start_stamp=200,
        exit_code=7,
        latest_failure_json=latest_failure_json,
        latest_failure_md=latest_failure_md,
        latest_run_json=latest_run_json,
        log_file=log_file,
        finish_stamp=260,
    )

    assert latest_failure_json.exists()
    failure_payload = json.loads(latest_failure_json.read_text(encoding="utf-8"))
    assert failure_payload["profile"] == "nightly"
    assert failure_payload["category"] == "runner_unknown_failure"
    assert failure_payload["error"] == "exit_code=7"
    assert failure_payload["artifacts"][0]["name"] == "runtime_log"
    assert failure_payload["artifacts"][0]["exists"] is True

    assert latest_failure_md.exists()
    md_text = latest_failure_md.read_text(encoding="utf-8")
    assert "- profile: nightly" in md_text
    assert "- exit_code: 7" in md_text

    run_payload = json.loads(latest_run_json.read_text(encoding="utf-8"))
    assert run_payload["status"] == "failed"
    assert run_payload["exit_code"] == 7
    assert run_payload["latest_failure_json"] == str(latest_failure_json)
    assert run_payload["latest_failure_markdown"] == str(latest_failure_md)
    assert run_payload["selected_failure_report_json"] == ""
    assert run_payload["selected_failure_report_json_stamp"] is None
    assert run_payload["selected_failure_report_markdown"] == ""
    assert run_payload["selected_failure_report_markdown_stamp"] is None


def test_finalize_gate_run_failure_prefers_newest_generated_reports(tmp_path: Path) -> None:
    module = _load_module()

    reports_dir = tmp_path / "reports"
    reports_dir.mkdir(parents=True, exist_ok=True)

    old_stamp = 300
    new_stamp = 450
    start_stamp = 320

    ignored_json = reports_dir / "xiuxian-daochang-memory-ci-failure-quick-200.json"
    old_json = reports_dir / f"xiuxian-daochang-memory-ci-failure-quick-{old_stamp}.json"
    new_json = reports_dir / f"xiuxian-daochang-memory-ci-failure-quick-{new_stamp}.json"
    old_md = reports_dir / f"xiuxian-daochang-memory-ci-failure-quick-{old_stamp}.md"
    new_md = reports_dir / f"xiuxian-daochang-memory-ci-failure-quick-{new_stamp}.md"

    ignored_json.write_text('{"category": "too_old"}\n', encoding="utf-8")
    old_json.write_text('{"category": "old"}\n', encoding="utf-8")
    new_json.write_text('{"category": "new"}\n', encoding="utf-8")
    old_md.write_text("# old\n", encoding="utf-8")
    new_md.write_text("# new\n", encoding="utf-8")

    latest_failure_json = tmp_path / "latest-failure.json"
    latest_failure_md = tmp_path / "latest-failure.md"
    latest_run_json = tmp_path / "latest-run.json"
    log_file = tmp_path / "runner.log"
    log_file.write_text("failed\n", encoding="utf-8")

    module.finalize_gate_run(
        reports_dir=reports_dir,
        profile="quick",
        start_stamp=start_stamp,
        exit_code=9,
        latest_failure_json=latest_failure_json,
        latest_failure_md=latest_failure_md,
        latest_run_json=latest_run_json,
        log_file=log_file,
        finish_stamp=500,
    )

    assert latest_failure_json.read_text(encoding="utf-8") == new_json.read_text(encoding="utf-8")
    assert latest_failure_md.read_text(encoding="utf-8") == new_md.read_text(encoding="utf-8")

    run_payload = json.loads(latest_run_json.read_text(encoding="utf-8"))
    assert run_payload["status"] == "failed"
    assert run_payload["selected_failure_report_json"] == str(new_json)
    assert run_payload["selected_failure_report_json_stamp"] == new_stamp
    assert run_payload["selected_failure_report_markdown"] == str(new_md)
    assert run_payload["selected_failure_report_markdown_stamp"] == new_stamp
