#!/usr/bin/env python3

from __future__ import annotations

import json
import os
import subprocess
import sys
from pathlib import Path

import pytest

PROJECT_ROOT = Path(__file__).resolve().parents[2]
LAUNCHER_SCRIPT = PROJECT_ROOT / "scripts" / "channel" / "start-xiuxian-daochang-memory-ci.sh"


def _run(tmp_path: Path, *args: str) -> subprocess.CompletedProcess[str]:
    runtime_root = tmp_path / ".run"
    runtime_root.mkdir(parents=True, exist_ok=True)
    env = dict(os.environ)
    env["PRJ_RUNTIME_DIR"] = str(runtime_root)
    return subprocess.run(
        ["bash", str(LAUNCHER_SCRIPT), *args],
        cwd=PROJECT_ROOT,
        env=env,
        text=True,
        capture_output=True,
        check=False,
    )


def test_launcher_help_succeeds(tmp_path: Path) -> None:
    result = _run(tmp_path, "--help")
    assert result.returncode == 0
    assert "start-xiuxian-daochang-memory-ci.sh" in result.stdout
    assert "--profile <quick|nightly>" in result.stdout


def test_launcher_requires_valid_profile(tmp_path: Path) -> None:
    result = _run(tmp_path, "--python-bin", sys.executable)
    assert result.returncode != 0
    assert "Invalid --profile" in result.stderr


def _run_profile_foreground(
    tmp_path: Path,
    profile: str,
    *gate_args: str,
) -> subprocess.CompletedProcess[str]:
    runtime_root = tmp_path / ".run"
    reports_dir = runtime_root / "reports"
    logs_dir = runtime_root / "logs"
    state_dir = runtime_root / "state"
    reports_dir.mkdir(parents=True, exist_ok=True)
    logs_dir.mkdir(parents=True, exist_ok=True)
    state_dir.mkdir(parents=True, exist_ok=True)

    latest_run_json = reports_dir / "latest-run.json"
    latest_failure_json = reports_dir / "latest-failure.json"
    latest_failure_md = reports_dir / "latest-failure.md"
    log_file = logs_dir / f"{profile}.log"
    pid_file = state_dir / f"{profile}.pid"

    env = dict(os.environ)
    env["PRJ_RUNTIME_DIR"] = str(runtime_root)
    return subprocess.run(
        [
            "bash",
            str(LAUNCHER_SCRIPT),
            "--profile",
            profile,
            "--foreground",
            "--python-bin",
            sys.executable,
            "--no-agent-bin-default",
            "--latest-run-json",
            str(latest_run_json),
            "--latest-failure-json",
            str(latest_failure_json),
            "--latest-failure-md",
            str(latest_failure_md),
            "--log-file",
            str(log_file),
            "--pid-file",
            str(pid_file),
            "--",
            *gate_args,
        ],
        cwd=PROJECT_ROOT,
        env=env,
        text=True,
        capture_output=True,
        check=False,
    )


@pytest.mark.parametrize(
    ("profile", "title"),
    [("quick", "Quick"), ("nightly", "Nightly")],
)
def test_launcher_foreground_help_updates_latest_run(
    tmp_path: Path,
    profile: str,
    title: str,
) -> None:
    result = _run_profile_foreground(tmp_path, profile, "--help")
    assert result.returncode == 0, result.stderr
    assert f"{title} memory CI gate completed successfully." in result.stdout

    latest_run_json = tmp_path / ".run" / "reports" / "latest-run.json"
    assert latest_run_json.exists()
    payload = json.loads(latest_run_json.read_text(encoding="utf-8"))
    assert payload["status"] == "passed"
    assert payload["exit_code"] == 0
    assert payload["profile"] == profile


@pytest.mark.parametrize(
    ("profile", "title"),
    [("quick", "Quick"), ("nightly", "Nightly")],
)
def test_launcher_foreground_invalid_gate_arg_writes_fallback_failure_payload(
    tmp_path: Path,
    profile: str,
    title: str,
) -> None:
    result = _run_profile_foreground(tmp_path, profile, "--definitely-invalid-flag")
    assert result.returncode != 0
    assert f"{title} memory CI gate failed with exit code" in result.stdout

    reports_dir = tmp_path / ".run" / "reports"
    latest_run_json = reports_dir / "latest-run.json"
    latest_failure_json = reports_dir / "latest-failure.json"
    latest_failure_md = reports_dir / "latest-failure.md"

    assert latest_run_json.exists()
    run_payload = json.loads(latest_run_json.read_text(encoding="utf-8"))
    assert run_payload["status"] == "failed"
    assert run_payload["exit_code"] != 0

    assert latest_failure_json.exists()
    failure_payload = json.loads(latest_failure_json.read_text(encoding="utf-8"))
    assert failure_payload["category"] == "runner_unknown_failure"
    assert "exit_code=" in failure_payload["error"]
    assert "repro_commands" in failure_payload

    assert latest_failure_md.exists()
