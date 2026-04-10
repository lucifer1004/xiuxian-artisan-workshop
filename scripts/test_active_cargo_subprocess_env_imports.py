from __future__ import annotations

import importlib.util
import sys
from pathlib import Path

import pytest

PROJECT_ROOT = Path(__file__).resolve().parents[1]
REMOVED_IMPORT = "xiuxian_wendao_py.compat.runtime"
TARGETS = [
    "scripts/benchmark_wendao_related.py",
    "scripts/benchmark_wendao_search.py",
    "scripts/evaluate_wendao_retrieval.py",
    "scripts/rust/cargo_check_with_timeout.py",
    "scripts/rust/xiuxian_daochang_embedding_role_perf_smoke.py",
    "scripts/test_llm_proxy.py",
    "scripts/validate_wendao_gate_reports.py",
]


@pytest.mark.parametrize("relative_path", TARGETS)
def test_active_script_imports_do_not_depend_on_removed_compat_runtime(
    relative_path: str,
) -> None:
    script_path = PROJECT_ROOT / relative_path
    source = script_path.read_text(encoding="utf-8")
    assert REMOVED_IMPORT not in source

    module_name = relative_path.replace("/", "_").removesuffix(".py")
    spec = importlib.util.spec_from_file_location(module_name, script_path)
    assert spec is not None
    assert spec.loader is not None
    module = importlib.util.module_from_spec(spec)
    sys.modules[spec.name] = module
    try:
        spec.loader.exec_module(module)
    finally:
        sys.modules.pop(spec.name, None)
