from __future__ import annotations

from pathlib import Path


PROJECT_ROOT = Path(__file__).resolve().parents[6]


def test_local_vector_service_modules_are_absent() -> None:
    assert not (
        PROJECT_ROOT
        / "packages/python/foundation/src/xiuxian_foundation/services/vector/__init__.py"
    ).exists()
    assert not (
        PROJECT_ROOT / "packages/python/foundation/src/xiuxian_foundation/services/vector/search.py"
    ).exists()
    assert not (
        PROJECT_ROOT
        / "packages/python/foundation/src/xiuxian_foundation/services/vector/constants.py"
    ).exists()
    assert not (
        PROJECT_ROOT / "packages/python/foundation/src/xiuxian_foundation/services/vector/models.py"
    ).exists()
    assert not (
        PROJECT_ROOT / "packages/python/foundation/src/xiuxian_foundation/services/vector_schema.py"
    ).exists()
    assert not (
        PROJECT_ROOT / "packages/python/foundation/tests/unit/services/test_vector_schema.py"
    ).exists()
    assert not (
        PROJECT_ROOT
        / "packages/python/foundation/tests/unit/services/test_vector_search_helpers.py"
    ).exists()


def test_tool_search_python_contract_surface_is_absent() -> None:
    assert not (
        PROJECT_ROOT
        / "packages/python/foundation/tests/unit/services/snapshots/tool_router_result_contract_v1.json"
    ).exists()


def test_local_retrieval_query_modules_are_absent() -> None:
    assert not (
        PROJECT_ROOT / "packages/python/foundation/src/xiuxian_rag/retrieval/__init__.py"
    ).exists()
    assert not (
        PROJECT_ROOT / "packages/python/foundation/src/xiuxian_rag/retrieval/interface.py"
    ).exists()
    assert not (
        PROJECT_ROOT / "packages/python/foundation/src/xiuxian_rag/retrieval/optimization.py"
    ).exists()
    assert not (
        PROJECT_ROOT / "packages/python/foundation/src/xiuxian_rag/retrieval/postprocess.py"
    ).exists()
    assert not (
        PROJECT_ROOT / "packages/python/foundation/src/xiuxian_rag/retrieval/response.py"
    ).exists()
    assert not (
        PROJECT_ROOT / "packages/python/foundation/src/xiuxian_rag/retrieval/rows.py"
    ).exists()
    assert not (
        PROJECT_ROOT / "packages/python/foundation/src/xiuxian_rag/retrieval/executor.py"
    ).exists()
    assert not (
        PROJECT_ROOT / "packages/python/foundation/src/xiuxian_rag/retrieval/config.py"
    ).exists()
    assert not (
        PROJECT_ROOT / "packages/python/foundation/src/xiuxian_rag/retrieval/errors.py"
    ).exists()
    assert not (
        PROJECT_ROOT / "packages/python/foundation/src/xiuxian_rag/retrieval/factory.py"
    ).exists()
    assert not (
        PROJECT_ROOT / "packages/python/foundation/src/xiuxian_rag/retrieval/hybrid.py"
    ).exists()
    assert not (
        PROJECT_ROOT / "packages/python/foundation/src/xiuxian_rag/retrieval/node_factory.py"
    ).exists()
    assert not (
        PROJECT_ROOT / "packages/python/foundation/src/xiuxian_rag/retrieval/normalize.py"
    ).exists()
    assert not (
        PROJECT_ROOT / "packages/python/foundation/src/xiuxian_rag/retrieval/single_call.py"
    ).exists()
    assert not (
        PROJECT_ROOT / "packages/python/foundation/tests/unit/rag/test_retrieval_architecture_rules.py"
    ).exists()
    assert not (
        PROJECT_ROOT / "packages/python/foundation/tests/unit/rag/test_retrieval_config.py"
    ).exists()
    assert not (
        PROJECT_ROOT / "packages/python/foundation/tests/unit/rag/test_retrieval_factory.py"
    ).exists()
    assert not (
        PROJECT_ROOT / "packages/python/foundation/tests/unit/rag/test_retrieval_executor.py"
    ).exists()
    assert not (
        PROJECT_ROOT / "packages/python/foundation/tests/unit/rag/test_retrieval_namespace.py"
    ).exists()
    assert not (
        PROJECT_ROOT / "packages/python/foundation/tests/unit/rag/test_retrieval_optimization.py"
    ).exists()
    assert not (
        PROJECT_ROOT / "packages/python/foundation/tests/unit/rag/test_retrieval_node_factory.py"
    ).exists()
    assert not (
        PROJECT_ROOT / "packages/python/foundation/tests/unit/rag/test_retrieval_normalize.py"
    ).exists()
    assert not (
        PROJECT_ROOT / "packages/python/foundation/tests/unit/rag/test_retrieval_postprocess.py"
    ).exists()
    assert not (
        PROJECT_ROOT / "packages/python/foundation/tests/unit/rag/test_retrieval_response.py"
    ).exists()
    assert not (
        PROJECT_ROOT / "packages/python/foundation/tests/unit/rag/test_retrieval_rows.py"
    ).exists()
    assert not (
        PROJECT_ROOT / "packages/python/foundation/tests/unit/rag/test_retrieval_single_call.py"
    ).exists()


def test_vector_contract_helper_module_is_absent() -> None:
    assert not (
        PROJECT_ROOT / "packages/python/foundation/tests/unit/services/_vector_payloads.py"
    ).exists()


def test_tracer_local_retrieval_invoker_surface_is_absent() -> None:
    tracer_init = (
        PROJECT_ROOT / "packages/python/foundation/src/xiuxian_tracer/__init__.py"
    ).read_text(encoding="utf-8")
    pipeline_schema = (
        PROJECT_ROOT / "packages/python/foundation/src/xiuxian_tracer/pipeline_schema.py"
    ).read_text(encoding="utf-8")
    pipeline_runtime = (
        PROJECT_ROOT / "packages/python/foundation/src/xiuxian_tracer/pipeline_runtime.py"
    ).read_text(encoding="utf-8")
    invoker_stack = (
        PROJECT_ROOT / "packages/python/foundation/src/xiuxian_tracer/invoker_stack.py"
    ).read_text(encoding="utf-8")

    assert not (
        PROJECT_ROOT / "packages/python/foundation/src/xiuxian_tracer/retrieval_invoker.py"
    ).exists()
    assert "RetrievalToolInvoker" not in tracer_init
    assert "InvokerRuntimeConfig" not in tracer_init
    assert "RetrievalRuntimeConfig" not in tracer_init
    assert "include_retrieval" not in pipeline_schema
    assert "default_backend" not in pipeline_schema
    assert "include_retrieval" not in pipeline_runtime
    assert "retrieval_default_backend" not in pipeline_runtime
    assert "RetrievalToolInvoker" not in invoker_stack
    assert not (
        PROJECT_ROOT / "packages/python/foundation/tests/unit/tracer/test_retrieval_invoker.py"
    ).exists()


def test_graph_enhancement_doc_no_longer_mentions_deleted_python_vector_search_module() -> None:
    graph_doc = (PROJECT_ROOT / "docs/01_core/wendao/graph-enhancement.md").read_text(
        encoding="utf-8"
    )

    assert "vector/search.py" not in graph_doc
