"""Regression tests for lazy import contracts in `xiuxian_rag` namespace."""

from __future__ import annotations

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


def test_omni_rag_loads_retrieval_module_on_attribute_access() -> None:
    payload = _run_python(
        textwrap.dedent(
            """
            import json
            import sys
            import xiuxian_rag as rag

            _ = rag.RetrievalConfig
            print(
                json.dumps(
                    {
                        "retrieval_loaded": "xiuxian_rag.retrieval" in sys.modules,
                        "analyzer_loaded": "xiuxian_rag.analyzer" in sys.modules,
                    }
                )
            )
            """
        )
    )
    assert payload == {
        "retrieval_loaded": True,
        "analyzer_loaded": False,
    }
