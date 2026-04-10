from __future__ import annotations

import importlib.util
import sys
from pathlib import Path

import pytest

PROJECT_ROOT = Path(__file__).resolve().parents[1]
REMOVED_NAMESPACE = "omni.foundation.services"
TARGETS = [
    "scripts/run_keyword_backend_llm_eval.py",
]


@pytest.mark.parametrize("relative_path", TARGETS)
def test_keyword_eval_scripts_use_current_foundation_namespace(relative_path: str) -> None:
    script_path = PROJECT_ROOT / relative_path
    source = script_path.read_text(encoding="utf-8")
    assert REMOVED_NAMESPACE not in source

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
