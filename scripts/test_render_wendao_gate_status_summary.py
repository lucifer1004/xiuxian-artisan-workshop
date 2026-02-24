from __future__ import annotations

import importlib.util
import json
from pathlib import Path

_MODULE_PATH = Path(__file__).resolve().with_name("render_wendao_gate_status_summary.py")
_MODULE_SPEC = importlib.util.spec_from_file_location(
    "render_wendao_gate_status_summary", _MODULE_PATH
)
assert _MODULE_SPEC is not None
assert _MODULE_SPEC.loader is not None
_MODULE = importlib.util.module_from_spec(_MODULE_SPEC)
_MODULE_SPEC.loader.exec_module(_MODULE)
render_gate_status_summary = _MODULE.render_gate_status_summary


def _write_json(path: Path, payload: dict[str, object]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(payload, ensure_ascii=True, indent=2) + "\n", encoding="utf-8")


def _retrieval_payload(top3_rate: float, *, error_count: int = 0) -> dict[str, object]:
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
        "thresholds": {
            "max_p95_ms": 50.0,
            "max_avg_ms": 30.0,
            "expect_subgraph_count_min": 1,
        },
        "gates_failed": gates_failed or [],
        "runs_detail": [],
    }


def test_render_wendao_gate_status_summary_green(tmp_path: Path) -> None:
    base_dir = tmp_path / "base"
    mixed_dir = tmp_path / "mixed"
    _write_json(base_dir / "retrieval_eval.json", _retrieval_payload(top3_rate=1.0))
    _write_json(base_dir / "related_benchmark.json", _related_payload(p95_ms=35.0))
    _write_json(mixed_dir / "retrieval_eval.json", _retrieval_payload(top3_rate=0.93))
    _write_json(mixed_dir / "related_benchmark.json", _related_payload(p95_ms=45.0))

    payload, errors = render_gate_status_summary(
        base_report_dir=base_dir,
        mixed_report_dir=mixed_dir,
        min_base_top3_rate=1.0,
        min_mixed_top3_rate=0.9,
        runner_os="linux",
    )

    assert errors == []
    assert payload["overall"]["ok"] is True
    base_line = payload["scopes"]["base"]["gate_log_line"]
    mixed_line = payload["scopes"]["mixed"]["gate_log_line"]
    assert "scope=base" in base_line
    assert "retrieval_ok=true" in base_line
    assert "related_ok=true" in base_line
    assert "scope=mixed" in mixed_line
    assert "top3_rate=0.9300" in mixed_line
    assert "blockers=none" in mixed_line


def test_render_wendao_gate_status_summary_flags_mixed_failures(tmp_path: Path) -> None:
    base_dir = tmp_path / "base"
    mixed_dir = tmp_path / "mixed"
    _write_json(base_dir / "retrieval_eval.json", _retrieval_payload(top3_rate=1.0))
    _write_json(base_dir / "related_benchmark.json", _related_payload())
    _write_json(mixed_dir / "retrieval_eval.json", _retrieval_payload(top3_rate=0.5))
    _write_json(
        mixed_dir / "related_benchmark.json",
        _related_payload(gates_failed=["p95_ms=99.0 > 50.0"]),
    )

    payload, errors = render_gate_status_summary(
        base_report_dir=base_dir,
        mixed_report_dir=mixed_dir,
        min_base_top3_rate=1.0,
        min_mixed_top3_rate=0.9,
        runner_os="linux",
    )

    assert errors == []
    assert payload["overall"]["ok"] is False
    mixed_scope = payload["scopes"]["mixed"]
    assert mixed_scope["ok"] is False
    assert "top3_below_threshold:0.5000<0.9000" in mixed_scope["blockers"]
    assert "related_gates_failed:1" in mixed_scope["blockers"]
    assert "retrieval_ok=false" in mixed_scope["gate_log_line"]
    assert "related_ok=false" in mixed_scope["gate_log_line"]
