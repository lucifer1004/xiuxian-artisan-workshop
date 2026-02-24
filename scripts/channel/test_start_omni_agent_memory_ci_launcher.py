#!/usr/bin/env python3

from __future__ import annotations

import os
import subprocess
import sys
from pathlib import Path

PROJECT_ROOT = Path(__file__).resolve().parents[2]
LAUNCHER_SCRIPT = PROJECT_ROOT / "scripts" / "channel" / "start-omni-agent-memory-ci.sh"


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
    assert "start-omni-agent-memory-ci.sh" in result.stdout
    assert "--profile <quick|nightly>" in result.stdout


def test_launcher_requires_valid_profile(tmp_path: Path) -> None:
    result = _run(tmp_path, "--python-bin", sys.executable)
    assert result.returncode != 0
    assert "Invalid --profile" in result.stderr
