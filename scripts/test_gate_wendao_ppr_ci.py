from __future__ import annotations

import json
import os
import stat
import subprocess
from pathlib import Path


def _write_executable(path: Path, content: str) -> None:
    path.write_text(content, encoding="utf-8")
    mode = path.stat().st_mode
    path.chmod(mode | stat.S_IXUSR | stat.S_IXGRP | stat.S_IXOTH)


def _write_stub_eval(path: Path) -> None:
    _write_executable(
        path,
        """#!/usr/bin/env python3
from __future__ import annotations
import argparse
import json
import os
import sys

parser = argparse.ArgumentParser()
parser.add_argument("--min-top3-rate", type=float, default=0.0)
parser.add_argument("--json", action="store_true")
args, _ = parser.parse_known_args()

top3 = float(os.environ.get("TEST_RETRIEVAL_TOP3", "1.0"))
payload = {
    "summary": {
        "schema": "xiuxian_wendao.retrieval_eval.v1",
        "total_cases": 10,
        "top1_count": 8,
        "top3_count": int(round(top3 * 10)),
        "top10_count": 10,
        "error_count": 0,
        "top1_rate": 0.8,
        "top3_rate": top3,
        "top10_rate": 1.0,
    },
    "failed_cases": [],
    "cases": [],
}
print(json.dumps(payload, ensure_ascii=True, indent=2))
if top3 < args.min_top3_rate:
    sys.exit(1)
""",
    )


def _write_stub_bench(path: Path) -> None:
    _write_executable(
        path,
        """#!/usr/bin/env python3
from __future__ import annotations
import argparse
import json
import os
import sys

parser = argparse.ArgumentParser()
parser.add_argument("--max-p95-ms", type=float, default=0.0)
parser.add_argument("--max-avg-ms", type=float, default=0.0)
parser.add_argument("--expect-subgraph-count-min", type=int, default=0)
parser.add_argument("--json", action="store_true")
args, _ = parser.parse_known_args()

p95 = float(os.environ.get("TEST_RELATED_P95_MS", "20.0"))
avg = float(os.environ.get("TEST_RELATED_AVG_MS", "12.0"))
avg_subgraph = float(os.environ.get("TEST_RELATED_AVG_SUBGRAPH_COUNT", "2.0"))
failed_runs = int(os.environ.get("TEST_RELATED_FAILED_RUNS", "0"))

gates_failed = []
if args.max_p95_ms > 0 and p95 > args.max_p95_ms:
    gates_failed.append(f"p95_ms={p95:.2f} > {args.max_p95_ms:.2f}")
if args.max_avg_ms > 0 and avg > args.max_avg_ms:
    gates_failed.append(f"avg_ms={avg:.2f} > {args.max_avg_ms:.2f}")
if args.expect_subgraph_count_min > 0 and avg_subgraph < args.expect_subgraph_count_min:
    gates_failed.append(
        f"avg_subgraph_count={avg_subgraph:.2f} < {args.expect_subgraph_count_min}"
    )
if failed_runs > 0:
    gates_failed.append(f"run_failures={failed_runs}")

payload = {
    "schema": "xiuxian_wendao.related_benchmark.v1",
    "profile": "debug",
    "summary": {
        "avg_ms": avg,
        "median_ms": avg,
        "p95_ms": p95,
        "min_ms": avg,
        "max_ms": p95,
        "ok_runs": 5 - failed_runs,
        "failed_runs": failed_runs,
        "avg_result_count": 10.0,
        "avg_subgraph_count": avg_subgraph,
        "avg_kernel_duration_ms": 3.0,
        "avg_partition_duration_ms": 2.0,
        "avg_fusion_duration_ms": 1.0,
        "avg_total_duration_ms": 8.0,
    },
    "thresholds": {
        "max_p95_ms": args.max_p95_ms,
        "max_avg_ms": args.max_avg_ms,
        "expect_subgraph_count_min": args.expect_subgraph_count_min,
    },
    "gates_failed": gates_failed,
    "runs_detail": [],
}
print(json.dumps(payload, ensure_ascii=True, indent=2))
if gates_failed:
    sys.exit(1)
""",
    )


def _run_gate(tmp_path: Path, *, retrieval_top3: float) -> subprocess.CompletedProcess[str]:
    repo_root = Path(__file__).resolve().parents[1]
    gate_script = repo_root / "scripts" / "gate_wendao_ppr.sh"
    eval_script = tmp_path / "stub_eval.py"
    bench_script = tmp_path / "stub_bench.py"
    matrix_file = tmp_path / "matrix.json"
    report_dir = tmp_path / "reports"

    _write_stub_eval(eval_script)
    _write_stub_bench(bench_script)
    matrix_file.write_text("{}", encoding="utf-8")

    env = os.environ.copy()
    env["XIUXIAN_WENDAO_GATE_EVAL_SCRIPT"] = str(eval_script)
    env["XIUXIAN_WENDAO_GATE_BENCH_SCRIPT"] = str(bench_script)
    env["XIUXIAN_WENDAO_GATE_BINARY"] = "/bin/true"
    env["XIUXIAN_WENDAO_GATE_REPORT_DIR"] = str(report_dir)
    env["TEST_RETRIEVAL_TOP3"] = str(retrieval_top3)
    env["TEST_RELATED_P95_MS"] = "20.0"
    env["TEST_RELATED_AVG_MS"] = "12.0"
    env["TEST_RELATED_AVG_SUBGRAPH_COUNT"] = "2.0"
    env["TEST_RELATED_FAILED_RUNS"] = "0"

    return subprocess.run(
        [
            "bash",
            str(gate_script),
            str(matrix_file),
            "10",
            "debug",
            "no-build",
            "1.0",
            "README",
            "3",
            "1",
            "auto",
            "1500",
            "1200",
            "1",
            "json",
        ],
        cwd=str(repo_root),
        env=env,
        check=False,
        capture_output=True,
        text=True,
    )


def test_gate_wendao_ppr_json_mode_emits_gate_line_and_reports(tmp_path: Path) -> None:
    result = _run_gate(tmp_path, retrieval_top3=1.0)
    assert result.returncode == 0
    assert "WENDAO_PPR_GATE retrieval_ok=true" in result.stderr
    assert "related_ok=true" in result.stderr
    assert "blockers=none" in result.stderr

    retrieval_report = tmp_path / "reports" / "retrieval_eval.json"
    related_report = tmp_path / "reports" / "related_benchmark.json"
    assert retrieval_report.exists()
    assert related_report.exists()
    retrieval_payload = json.loads(retrieval_report.read_text(encoding="utf-8"))
    related_payload = json.loads(related_report.read_text(encoding="utf-8"))
    assert retrieval_payload["summary"]["schema"] == "xiuxian_wendao.retrieval_eval.v1"
    assert related_payload["schema"] == "xiuxian_wendao.related_benchmark.v1"


def test_gate_wendao_ppr_json_mode_still_emits_gate_line_when_retrieval_fails(
    tmp_path: Path,
) -> None:
    result = _run_gate(tmp_path, retrieval_top3=0.4)
    assert result.returncode == 1
    assert "WENDAO_PPR_GATE retrieval_ok=false" in result.stderr
    assert "related_ok=true" in result.stderr
    assert "retrieval_gate_failed:rc=1" in result.stderr

    retrieval_report = tmp_path / "reports" / "retrieval_eval.json"
    assert retrieval_report.exists()
    retrieval_payload = json.loads(retrieval_report.read_text(encoding="utf-8"))
    assert retrieval_payload["summary"]["top3_rate"] == 0.4
