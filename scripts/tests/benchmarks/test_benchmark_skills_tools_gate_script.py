"""Unit tests for scripts/benchmark_skills_tools_gate.sh."""

from __future__ import annotations

import os
import subprocess
from typing import TYPE_CHECKING

from xiuxian_foundation.config.prj import get_project_root

if TYPE_CHECKING:
    from pathlib import Path


def _gate_script_path() -> Path:
    return get_project_root() / "scripts" / "benchmark_skills_tools_gate.sh"


def _run_gate_dry(mode: str, runs: str = "") -> subprocess.CompletedProcess[str]:
    script = _gate_script_path()
    env = dict(os.environ)
    env["OMNI_SKILLS_TOOLS_GATE_DRY_RUN"] = "1"
    cmd = ["bash", str(script), mode]
    if runs:
        cmd.append(runs)
    return subprocess.run(cmd, check=False, capture_output=True, text=True, env=env)


def test_gate_script_deterministic_builds_expected_command() -> None:
    result = _run_gate_dry("deterministic", "3")
    assert result.returncode == 0
    out = result.stdout
    assert "scripts/benchmark_skills_tools.py" in out
    assert "--runs 3" in out
    assert "--crawl4ai-scenarios local" in out
    assert "--snapshot-gate-scope deterministic" in out
    assert "--enforce-cli-ordering" in out
    assert "--cli-ordering-tolerance-ms 50" in out
    assert "--snapshot-default-metric p50" in out
    assert "--snapshot-network-metric trimmed_avg" in out
    assert "--strict-snapshot" in out


def test_gate_script_network_builds_expected_command() -> None:
    result = _run_gate_dry("network", "5")
    assert result.returncode == 0
    out = result.stdout
    assert "scripts/benchmark_skills_tools.py" in out
    assert "--runs 5" in out
    assert "--tools crawl4ai.crawl_url" in out
    assert "--crawl4ai-scenarios both" in out
    assert "--snapshot-gate-scope all" in out
    assert "--enforce-cli-ordering" not in out
    assert "--snapshot-default-metric p50" in out
    assert "--snapshot-network-metric trimmed_avg" in out
    assert "--strict-snapshot" not in out


def test_gate_script_rejects_invalid_mode() -> None:
    script = _gate_script_path()
    result = subprocess.run(
        ["bash", str(script), "invalid-mode"],
        check=False,
        capture_output=True,
        text=True,
        env=dict(os.environ),
    )
    assert result.returncode == 2
    assert "Invalid mode" in result.stderr
