"""Architecture guardrails for retrieval namespace layering."""

from __future__ import annotations

import ast
import inspect
from pathlib import Path

import pytest

from xiuxian_rag.retrieval.factory import create_retrieval_backend
from xiuxian_rag.retrieval.hybrid import HybridRetrievalBackend

RAG_RETRIEVAL_DIR = Path(__file__).resolve().parents[3] / "src" / "omni" / "rag" / "retrieval"
pytestmark = pytest.mark.architecture


def _parse(module_name: str) -> ast.Module:
    path = RAG_RETRIEVAL_DIR / module_name
    return ast.parse(path.read_text(encoding="utf-8"), filename=str(path))


def _imported_modules(tree: ast.Module) -> list[str]:
    modules: list[str] = []
    for node in tree.body:
        if isinstance(node, ast.ImportFrom):
            modules.append(node.module or "")
        elif isinstance(node, ast.Import):
            for alias in node.names:
                modules.append(alias.name)
    return modules


def test_interface_module_has_no_backend_dependencies() -> None:
    imported = _imported_modules(_parse("interface.py"))
    forbidden = {"lancedb", "hybrid", "factory", "node_factory"}
    for module in imported:
        assert not any(name in module for name in forbidden), (
            "interface.py must not import backend/factory modules"
        )


def test_backend_modules_depend_on_interface_layer() -> None:
    for module_name in ["hybrid.py"]:
        imported = _imported_modules(_parse(module_name))
        assert any("interface" in module for module in imported), (
            f"{module_name} must import retrieval interface layer"
        )


def test_local_lancedb_backend_module_is_removed() -> None:
    assert not (RAG_RETRIEVAL_DIR / "lancedb.py").exists()


def test_factory_module_does_not_depend_on_tracer_or_pipeline() -> None:
    imported = _imported_modules(_parse("factory.py"))
    forbidden = {"xiuxian_tracer", "pipeline_", "workflow_engine", "graphflow"}
    for module in imported:
        assert not any(name in module for name in forbidden), (
            "factory.py must remain retrieval-only without tracer/pipeline coupling"
        )


def test_node_factory_depends_only_on_interface_contract() -> None:
    imported = _imported_modules(_parse("node_factory.py"))
    assert any("interface" in module for module in imported)
    forbidden = {"lancedb", "hybrid", "xiuxian_tracer"}
    for module in imported:
        assert not any(name in module for name in forbidden), (
            "node_factory.py must depend on interface contracts, not concrete backends"
        )


def test_hybrid_backend_has_no_python_fusion_method() -> None:
    """Hybrid fusion/scoring must remain Rust-owned."""
    assert not hasattr(HybridRetrievalBackend, "_rrf_fuse")


def test_factory_signature_has_no_keyword_backend_param() -> None:
    """Public API should enforce Rust-only hybrid entrypoints."""
    sig = inspect.signature(create_retrieval_backend)
    assert "keyword_backend" not in sig.parameters
    assert "reranker" not in sig.parameters
