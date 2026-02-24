"""Unit tests for scripts/ci-local-recall-gates.sh."""

from __future__ import annotations

import os
import subprocess
from typing import TYPE_CHECKING

from omni.foundation.runtime.gitops import get_project_root

if TYPE_CHECKING:
    from pathlib import Path


def _script_path() -> Path:
    return get_project_root() / "scripts" / "ci-local-recall-gates.sh"


def _run_dry(
    *args: str, env_overrides: dict[str, str] | None = None
) -> subprocess.CompletedProcess[str]:
    env = dict(os.environ)
    env["OMNI_RECALL_GATES_DRY_RUN"] = "1"
    if env_overrides:
        env.update(env_overrides)
    return subprocess.run(
        ["bash", str(_script_path()), *args],
        check=False,
        capture_output=True,
        text=True,
        env=env,
    )


def test_recall_gates_dry_run_defaults_include_auto_and_graph_modes() -> None:
    result = _run_dry()
    assert result.returncode == 0
    out = result.stdout
    assert "mkdir -p .run/reports/knowledge-recall-perf" in out
    assert "--retrieval-mode auto" in out
    assert "--json-output .run/reports/knowledge-recall-perf/auto.json" in out
    assert "--retrieval-mode graph_only" in out
    assert "--json-output .run/reports/knowledge-recall-perf/graph_only.json" in out


def test_recall_gates_dry_run_honors_args_and_threshold_envs() -> None:
    result = _run_dry(
        "7",
        "2",
        "architecture",
        "4",
        "/tmp/recall-reports",
        env_overrides={
            "OMNI_KNOWLEDGE_RECALL_P95_MS": "1111",
            "OMNI_KNOWLEDGE_RECALL_GRAPH_P95_MS": "777",
        },
    )
    assert result.returncode == 0
    out = result.stdout
    assert "mkdir -p /tmp/recall-reports" in out
    assert "--runs 7" in out
    assert "--warm-runs 2" in out
    assert "--query architecture" in out
    assert "--limit 4" in out
    assert "--max-p95-ms 1111" in out
    assert "--max-p95-ms 777" in out
