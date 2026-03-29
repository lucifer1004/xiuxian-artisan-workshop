from __future__ import annotations

from pathlib import Path


PROJECT_ROOT = Path(__file__).resolve().parents[6]


def test_python_agent_legacy_scripts_are_absent() -> None:
    removed = [
        "scripts/benchmark_kernel.py",
        "scripts/benchmark_knowledge_recall.py",
        "scripts/benchmark_skills_tools.py",
        "scripts/ci-local-recall-gates.sh",
        "scripts/knowledge_recall_perf_gate.py",
        "scripts/recall_profile.py",
        "scripts/skills_monitor.py",
        "scripts/visual_stress_test.py",
        "scripts/verify_rust_pruner.py",
        "scripts/verify_recall.py",
        "scripts/verify_generator.py",
        "scripts/test_render.py",
        "scripts/verify_archiver.py",
        "scripts/recall_profile_phases.py",
    ]

    for relative_path in removed:
        assert not (PROJECT_ROOT / relative_path).exists(), relative_path
