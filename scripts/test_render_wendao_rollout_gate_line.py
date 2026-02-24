from __future__ import annotations

import json
import subprocess
from pathlib import Path


def _write_json(path: Path, payload: dict[str, object]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(payload, ensure_ascii=True, indent=2) + "\n", encoding="utf-8")


def test_render_wendao_rollout_gate_line_uses_gate_log_line_when_present(tmp_path: Path) -> None:
    repo_root = Path(__file__).resolve().parents[1]
    script_path = repo_root / "scripts" / "render_wendao_rollout_gate_line.py"
    status_path = tmp_path / "wendao_rollout_status.json"
    _write_json(
        status_path,
        {
            "readiness": {
                "ready": True,
                "remaining_consecutive_runs": 0,
                "blockers": [],
                "gate_log_line": "WENDAO_ROLLOUT ready=true streak=7/7 remaining=0 blockers=none",
            },
            "streaks": {"both_ok": 7},
            "criteria": {"required_consecutive_runs": 7},
        },
    )

    result = subprocess.run(
        ["uv", "run", "python", str(script_path), "--status-json", str(status_path)],
        cwd=str(repo_root),
        check=False,
        capture_output=True,
        text=True,
    )

    assert result.returncode == 0
    assert (
        result.stdout.strip() == "1\tWENDAO_ROLLOUT ready=true streak=7/7 remaining=0 blockers=none"
    )


def test_render_wendao_rollout_gate_line_falls_back_when_gate_log_line_missing(
    tmp_path: Path,
) -> None:
    repo_root = Path(__file__).resolve().parents[1]
    script_path = repo_root / "scripts" / "render_wendao_rollout_gate_line.py"
    status_path = tmp_path / "wendao_rollout_status.json"
    _write_json(
        status_path,
        {
            "readiness": {
                "ready": False,
                "remaining_consecutive_runs": 6,
                "blockers": ["mixed_canary_not_green", "consecutive_runs_remaining:6"],
                "gate_log_line": "",
            },
            "streaks": {"both_ok": 1},
            "criteria": {"required_consecutive_runs": 7},
        },
    )

    result = subprocess.run(
        ["uv", "run", "python", str(script_path), "--status-json", str(status_path)],
        cwd=str(repo_root),
        check=False,
        capture_output=True,
        text=True,
    )

    assert result.returncode == 0
    assert (
        result.stdout.strip()
        == "0\tWENDAO_ROLLOUT ready=false streak=1/7 remaining=6 blockers=mixed_canary_not_green|consecutive_runs_remaining:6"
    )
