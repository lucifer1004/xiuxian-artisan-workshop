"""End-to-end CLI integration tests for the persistent skill runner daemon."""

from __future__ import annotations

import json
import os
import shutil
import subprocess
import time
import uuid
from pathlib import Path

import pytest


@pytest.fixture
def repo_root() -> Path:
    """Return repository root path."""
    return Path(__file__).resolve().parents[5]


@pytest.fixture
def isolated_runner_socket() -> Path:
    """Return a per-test runner socket path under /tmp."""
    return (Path("/tmp") / f"omni-skill-runner-{uuid.uuid4().hex}.sock").resolve()


def _ensure_omni_available() -> str:
    omni = shutil.which("omni")
    if not omni:
        pytest.skip("omni CLI is not available in PATH")
    return omni


def _runner_env(socket_path: Path) -> dict[str, str]:
    env = os.environ.copy()
    env["OMNI_SKILL_RUNNER_SOCKET"] = str(socket_path)
    return env


def _run_omni(
    *,
    repo_root: Path,
    args: list[str],
    env: dict[str, str],
    timeout_seconds: float = 30.0,
) -> subprocess.CompletedProcess[str]:
    omni = _ensure_omni_available()
    return subprocess.run(
        [omni, *args],
        cwd=repo_root,
        env=env,
        text=True,
        capture_output=True,
        check=False,
        timeout=timeout_seconds,
    )


def _parse_json_stdout(result: subprocess.CompletedProcess[str]) -> dict[str, object]:
    text = result.stdout.strip()
    try:
        parsed = json.loads(text) if text else {}
    except json.JSONDecodeError as exc:
        raise AssertionError(
            f"stdout is not valid JSON\nreturncode={result.returncode}\nstdout={result.stdout}\n"
            f"stderr={result.stderr}"
        ) from exc
    if not isinstance(parsed, dict):
        raise AssertionError(f"expected JSON object, got: {type(parsed)}")
    return parsed


def _wait_for_socket_absent(socket_path: Path, *, timeout_seconds: float = 3.0) -> bool:
    deadline = time.monotonic() + timeout_seconds
    while time.monotonic() < deadline:
        if not socket_path.exists():
            return True
        time.sleep(0.05)
    return not socket_path.exists()


def _stop_runner_best_effort(*, repo_root: Path, env: dict[str, str]) -> None:
    _run_omni(
        repo_root=repo_root,
        args=["skill", "runner", "stop", "--json"],
        env=env,
        timeout_seconds=10.0,
    )


def test_skill_runner_lifecycle_e2e_cli(repo_root: Path, isolated_runner_socket: Path) -> None:
    """Runner lifecycle commands should work over the real CLI entrypoint."""
    env = _runner_env(isolated_runner_socket)
    _stop_runner_best_effort(repo_root=repo_root, env=env)

    try:
        status_before = _run_omni(
            repo_root=repo_root,
            args=["skill", "runner", "status", "--json"],
            env=env,
        )
        assert status_before.returncode == 0, status_before.stderr
        status_before_payload = _parse_json_stdout(status_before)
        assert status_before_payload.get("running") is False

        start = _run_omni(
            repo_root=repo_root,
            args=["skill", "runner", "start", "--json"],
            env=env,
        )
        assert start.returncode == 0, start.stderr
        start_payload = _parse_json_stdout(start)
        assert start_payload.get("running") is True
        assert isinstance(start_payload.get("pid"), int)

        status_running = _run_omni(
            repo_root=repo_root,
            args=["skill", "runner", "status", "--json"],
            env=env,
        )
        assert status_running.returncode == 0, status_running.stderr
        status_running_payload = _parse_json_stdout(status_running)
        assert status_running_payload.get("running") is True
        assert status_running_payload.get("pid") == start_payload.get("pid")

        stop = _run_omni(
            repo_root=repo_root,
            args=["skill", "runner", "stop", "--json"],
            env=env,
        )
        assert stop.returncode == 0, stop.stderr
        stop_payload = _parse_json_stdout(stop)
        assert stop_payload.get("stopped") is True

        status_after = _run_omni(
            repo_root=repo_root,
            args=["skill", "runner", "status", "--json"],
            env=env,
        )
        assert status_after.returncode == 0, status_after.stderr
        status_after_payload = _parse_json_stdout(status_after)
        assert status_after_payload.get("running") is False
        assert _wait_for_socket_absent(isolated_runner_socket)
    finally:
        _stop_runner_best_effort(repo_root=repo_root, env=env)


def test_skill_run_reuse_process_e2e_cli(repo_root: Path, isolated_runner_socket: Path) -> None:
    """`skill run --reuse-process` should auto-start and reuse the local daemon."""
    env = _runner_env(isolated_runner_socket)
    _stop_runner_best_effort(repo_root=repo_root, env=env)

    try:
        cold_run = _run_omni(
            repo_root=repo_root,
            args=[
                "skill",
                "run",
                "demo.hello",
                '{"name":"daemon-cold"}',
                "--json",
                "--reuse-process",
            ],
            env=env,
        )
        assert cold_run.returncode == 0, cold_run.stderr
        cold_payload = _parse_json_stdout(cold_run)
        assert "daemon-cold" in str(cold_payload.get("message", ""))

        status = _run_omni(
            repo_root=repo_root,
            args=["skill", "runner", "status", "--json"],
            env=env,
        )
        assert status.returncode == 0, status.stderr
        status_payload = _parse_json_stdout(status)
        assert status_payload.get("running") is True
        assert isinstance(status_payload.get("pid"), int)
        daemon_pid = status_payload.get("pid")

        warm_run = _run_omni(
            repo_root=repo_root,
            args=[
                "skill",
                "run",
                "demo.hello",
                '{"name":"daemon-warm"}',
                "--json",
                "--reuse-process",
            ],
            env=env,
        )
        assert warm_run.returncode == 0, warm_run.stderr
        warm_payload = _parse_json_stdout(warm_run)
        assert "daemon-warm" in str(warm_payload.get("message", ""))

        status_after = _run_omni(
            repo_root=repo_root,
            args=["skill", "runner", "status", "--json"],
            env=env,
        )
        assert status_after.returncode == 0, status_after.stderr
        status_after_payload = _parse_json_stdout(status_after)
        assert status_after_payload.get("running") is True
        assert status_after_payload.get("pid") == daemon_pid
    finally:
        _stop_runner_best_effort(repo_root=repo_root, env=env)
        assert _wait_for_socket_absent(isolated_runner_socket)
