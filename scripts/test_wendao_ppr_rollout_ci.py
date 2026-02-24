from __future__ import annotations

import json
import os
import subprocess
from pathlib import Path


def _write_json(path: Path, payload: dict[str, object]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(payload, ensure_ascii=True, indent=2) + "\n", encoding="utf-8")


def _retrieval_payload(top3_rate: float) -> dict[str, object]:
    return {
        "summary": {
            "schema": "xiuxian_wendao.retrieval_eval.v1",
            "total_cases": 10,
            "top1_count": 8,
            "top3_count": 10,
            "top10_count": 10,
            "error_count": 0,
            "top1_rate": 0.8,
            "top3_rate": top3_rate,
            "top10_rate": 1.0,
        },
        "failed_cases": [],
        "cases": [],
    }


def _related_payload() -> dict[str, object]:
    return {
        "schema": "xiuxian_wendao.related_benchmark.v1",
        "profile": "debug",
        "summary": {
            "avg_ms": 12.0,
            "median_ms": 11.0,
            "p95_ms": 20.0,
            "min_ms": 10.0,
            "max_ms": 24.0,
            "ok_runs": 5,
            "failed_runs": 0,
            "avg_result_count": 10.0,
            "avg_subgraph_count": 2.0,
            "avg_kernel_duration_ms": 3.0,
            "avg_partition_duration_ms": 2.0,
            "avg_fusion_duration_ms": 1.0,
            "avg_total_duration_ms": 8.0,
        },
        "thresholds": {
            "max_p95_ms": 50.0,
            "max_avg_ms": 30.0,
            "expect_subgraph_count_min": 1,
        },
        "gates_failed": [],
        "runs_detail": [],
    }


def _prepare_reports(base_dir: Path, mixed_dir: Path, *, mixed_top3_rate: float) -> None:
    _write_json(base_dir / "retrieval_eval.json", _retrieval_payload(top3_rate=1.0))
    _write_json(base_dir / "related_benchmark.json", _related_payload())
    _write_json(mixed_dir / "retrieval_eval.json", _retrieval_payload(top3_rate=mixed_top3_rate))
    _write_json(mixed_dir / "related_benchmark.json", _related_payload())


def _run_rollout_ci(
    base_dir: Path, mixed_dir: Path, *, strict: bool
) -> subprocess.CompletedProcess[str]:
    repo_root = Path(__file__).resolve().parents[1]
    script_path = repo_root / "scripts" / "wendao_ppr_rollout_ci.sh"
    env = os.environ.copy()
    env["XIUXIAN_WENDAO_ROLLOUT_FETCH_REMOTE_STATUS"] = "0"

    return subprocess.run(
        [
            "bash",
            str(script_path),
            str(base_dir),
            str(mixed_dir),
            "7",
            "0.9",
            "1" if strict else "0",
        ],
        cwd=str(repo_root),
        env=env,
        check=False,
        capture_output=True,
        text=True,
    )


def test_wendao_rollout_ci_emits_gate_line_and_json_in_advisory_mode(tmp_path: Path) -> None:
    base_dir = tmp_path / "base"
    mixed_dir = tmp_path / "mixed"
    _prepare_reports(base_dir, mixed_dir, mixed_top3_rate=0.92)

    result = _run_rollout_ci(base_dir, mixed_dir, strict=False)
    assert result.returncode == 0
    assert "WENDAO_ROLLOUT ready=false streak=1/7 remaining=6" in result.stderr

    payload = json.loads(result.stdout)
    assert payload["schema"] == "xiuxian_wendao.rollout_status.v1"
    assert payload["readiness"]["ready"] is False
    assert payload["readiness"]["remaining_consecutive_runs"] == 6
    assert payload["readiness"]["gate_log_line"].startswith("WENDAO_ROLLOUT ")

    saved_payload = json.loads(
        (base_dir / "wendao_rollout_status.json").read_text(encoding="utf-8")
    )
    assert saved_payload["readiness"]["gate_log_line"] == payload["readiness"]["gate_log_line"]
    gate_summary_payload = json.loads(
        (base_dir / "wendao_gate_status_summary.json").read_text(encoding="utf-8")
    )
    assert gate_summary_payload["schema"] == "xiuxian_wendao.gate_status_summary.v1"
    consolidated_md = (base_dir / "wendao_gate_rollout_status.md").read_text(encoding="utf-8")
    assert "## Wendao Gate Summary" in consolidated_md
    assert "# Wendao Rollout Status" in consolidated_md


def test_wendao_rollout_ci_strict_mode_fails_after_emitting_status(tmp_path: Path) -> None:
    base_dir = tmp_path / "base"
    mixed_dir = tmp_path / "mixed"
    _prepare_reports(base_dir, mixed_dir, mixed_top3_rate=0.92)

    result = _run_rollout_ci(base_dir, mixed_dir, strict=True)
    assert result.returncode == 1
    assert "WENDAO_ROLLOUT ready=false streak=1/7 remaining=6" in result.stderr

    payload = json.loads(result.stdout)
    assert payload["readiness"]["ready"] is False
    assert payload["readiness"]["gate_log_line"].startswith("WENDAO_ROLLOUT ")
    assert (base_dir / "wendao_gate_rollout_status.md").exists()


def test_wendao_rollout_ci_strict_mode_succeeds_when_ready(tmp_path: Path) -> None:
    base_dir = tmp_path / "base"
    mixed_dir = tmp_path / "mixed"
    _prepare_reports(base_dir, mixed_dir, mixed_top3_rate=0.95)
    _write_json(
        base_dir / "wendao_rollout_status.previous.json",
        {
            "schema": "xiuxian_wendao.rollout_status.v1",
            "streaks": {
                "base_gate_ok": 6,
                "mixed_canary_ok": 6,
                "report_validation_ok": 6,
                "both_ok": 6,
            },
        },
    )

    result = _run_rollout_ci(base_dir, mixed_dir, strict=True)
    assert result.returncode == 0
    assert "WENDAO_ROLLOUT ready=true streak=7/7 remaining=0 blockers=none" in result.stderr

    payload = json.loads(result.stdout)
    assert payload["readiness"]["ready"] is True
    assert payload["readiness"]["remaining_consecutive_runs"] == 0
    assert payload["readiness"]["gate_log_line"] == (
        "WENDAO_ROLLOUT ready=true streak=7/7 remaining=0 blockers=none"
    )
    consolidated_md = (base_dir / "wendao_gate_rollout_status.md").read_text(encoding="utf-8")
    assert "## Wendao Gate Summary" in consolidated_md
    assert "# Wendao Rollout Status" in consolidated_md
