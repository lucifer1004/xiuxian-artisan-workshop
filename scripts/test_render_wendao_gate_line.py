from __future__ import annotations

import json
import subprocess
from pathlib import Path


def _write_json(path: Path, payload: dict[str, object]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(payload, ensure_ascii=True, indent=2) + "\n", encoding="utf-8")


def _retrieval_payload(*, top3_rate: float = 1.0, error_count: int = 0) -> dict[str, object]:
    return {
        "summary": {
            "schema": "xiuxian_wendao.retrieval_eval.v1",
            "total_cases": 10,
            "top1_count": 8,
            "top3_count": round(top3_rate * 10),
            "top10_count": 10,
            "error_count": error_count,
            "top1_rate": 0.8,
            "top3_rate": top3_rate,
            "top10_rate": 1.0,
        },
        "failed_cases": [],
        "cases": [],
    }


def _related_payload(
    *, p95_ms: float = 20.0, gates_failed: list[str] | None = None
) -> dict[str, object]:
    return {
        "schema": "xiuxian_wendao.related_benchmark.v1",
        "profile": "debug",
        "summary": {
            "avg_ms": 12.0,
            "median_ms": 11.0,
            "p95_ms": p95_ms,
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
        "thresholds": {"max_p95_ms": 50.0, "max_avg_ms": 30.0, "expect_subgraph_count_min": 1},
        "gates_failed": gates_failed or [],
        "runs_detail": [],
    }


def test_render_wendao_gate_line_green(tmp_path: Path) -> None:
    repo_root = Path(__file__).resolve().parents[1]
    script_path = repo_root / "scripts" / "render_wendao_gate_line.py"
    retrieval_path = tmp_path / "retrieval_eval.json"
    related_path = tmp_path / "related_benchmark.json"
    _write_json(retrieval_path, _retrieval_payload(top3_rate=1.0, error_count=0))
    _write_json(related_path, _related_payload(p95_ms=18.5))

    result = subprocess.run(
        [
            "uv",
            "run",
            "python",
            str(script_path),
            "--retrieval-report",
            str(retrieval_path),
            "--related-report",
            str(related_path),
            "--min-top3-rate",
            "1.0",
            "--retrieval-exit-code",
            "0",
            "--related-exit-code",
            "0",
        ],
        cwd=str(repo_root),
        check=False,
        capture_output=True,
        text=True,
    )

    assert result.returncode == 0
    line = result.stdout.strip()
    assert "WENDAO_PPR_GATE " in line
    assert "retrieval_ok=true" in line
    assert "top3_rate=1.0000" in line
    assert "related_ok=true" in line
    assert "related_p95_ms=18.50" in line
    assert "related_gates_failed=0" in line
    assert "blockers=none" in line


def test_render_wendao_gate_line_includes_missing_report_and_exit_code_blockers(
    tmp_path: Path,
) -> None:
    repo_root = Path(__file__).resolve().parents[1]
    script_path = repo_root / "scripts" / "render_wendao_gate_line.py"
    retrieval_path = tmp_path / "retrieval_eval.json"
    missing_related = tmp_path / "missing_related_benchmark.json"
    _write_json(retrieval_path, _retrieval_payload(top3_rate=0.6, error_count=1))

    result = subprocess.run(
        [
            "uv",
            "run",
            "python",
            str(script_path),
            "--retrieval-report",
            str(retrieval_path),
            "--related-report",
            str(missing_related),
            "--min-top3-rate",
            "1.0",
            "--retrieval-exit-code",
            "1",
            "--related-exit-code",
            "2",
        ],
        cwd=str(repo_root),
        check=False,
        capture_output=True,
        text=True,
    )

    assert result.returncode == 0
    line = result.stdout.strip()
    assert "retrieval_ok=false" in line
    assert "related_ok=false" in line
    assert "related_gates_failed=0" in line
    assert "related_report_missing" in line
    assert "retrieval_gate_failed:rc=1" in line
    assert "related_gate_failed:rc=2" in line
