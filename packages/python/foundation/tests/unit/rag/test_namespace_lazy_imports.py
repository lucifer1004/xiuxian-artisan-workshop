"""Regression tests for lazy import contracts in `xiuxian_rag` namespace."""

from __future__ import annotations

import importlib
import importlib.util
import json
import subprocess
import sys
import textwrap


def _run_python(code: str) -> dict[str, object]:
    result = subprocess.run(
        [sys.executable, "-c", code],
        check=True,
        capture_output=True,
        text=True,
    )
    return json.loads(result.stdout.strip())


def test_omni_rag_import_is_lazy() -> None:
    payload = _run_python(
        textwrap.dedent(
            """
            import json
            import sys
            import xiuxian_rag

            watched = [
                "xiuxian_rag.analyzer",
                "xiuxian_rag.graph",
                "xiuxian_rag.multimodal",
                "xiuxian_rag.retrieval",
            ]
            print(json.dumps({name: (name in sys.modules) for name in watched}))
            """
        )
    )
    assert payload == {
        "xiuxian_rag.analyzer": False,
        "xiuxian_rag.graph": False,
        "xiuxian_rag.multimodal": False,
        "xiuxian_rag.retrieval": False,
    }


def test_omni_rag_root_missing_facade_does_not_load_retrieval_module() -> None:
    payload = _run_python(
        textwrap.dedent(
            """
            import json
            import sys
            import xiuxian_rag as rag

            try:
                _ = rag.RetrievalConfig
            except AttributeError:
                missing = True
            else:
                missing = False
            print(
                json.dumps(
                    {
                        "missing": missing,
                        "retrieval_loaded": "xiuxian_rag.retrieval" in sys.modules,
                        "analyzer_loaded": "xiuxian_rag.analyzer" in sys.modules,
                    }
                )
            )
            """
        )
    )
    assert payload == {
        "missing": True,
        "retrieval_loaded": False,
        "analyzer_loaded": False,
    }


def test_omni_rag_retrieval_package_is_absent() -> None:
    importlib.invalidate_caches()
    assert importlib.util.find_spec("xiuxian_rag.retrieval") is None
