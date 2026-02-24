"""Tests for common single-call recall orchestration."""

from __future__ import annotations

import sys
import types
from unittest.mock import AsyncMock

import pytest

from omni.rag.retrieval import run_recall_single_call


class _VectorStore:
    def get_store_for_collection(self, collection: str) -> object:
        assert collection == "knowledge_chunks"
        return object()


async def _identity_postprocess(
    rows: list[dict[str, object]],
    _query: str,
    _limit: int,
    _min_score: float,
    _preview: bool,
    _snippet_chars: int,
    _apply_fusion_boost: bool,
) -> list[dict[str, object]]:
    return list(rows)


@pytest.mark.asyncio
async def test_run_recall_single_call_prefers_graph_rows_without_vector_query(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    async def _fake_policy(*_args, **_kwargs):
        return types.SimpleNamespace(
            retrieval_path="graph_only",
            retrieval_reason="graph_sufficient",
            graph_backend="fake",
            graph_hit_count=1,
            graph_confidence_score=0.91,
            graph_confidence_level="high",
            retrieval_plan_schema_id="schema-id",
            retrieval_plan={"schema": "omni.link_graph.retrieval_plan.v1"},
            graph_rows=(
                {
                    "content": "graph row",
                    "source": "docs/a.md",
                    "score": 0.9,
                    "title": "",
                    "section": "",
                },
            ),
            graph_only_empty=False,
        )

    monkeypatch.setitem(
        sys.modules,
        "omni.rag.link_graph",
        types.SimpleNamespace(evaluate_link_graph_recall_policy=_fake_policy),
    )

    query_rows_runner = AsyncMock(return_value=[])
    response = await run_recall_single_call(
        vector_store=_VectorStore(),
        query="architecture",
        keywords=[],
        collection="knowledge_chunks",
        limit=5,
        fetch_limit=5,
        min_score=0.0,
        preview=False,
        snippet_chars=150,
        retrieval_mode="hybrid",
        postprocess_rows=_identity_postprocess,
        query_rows_runner=query_rows_runner,
    )

    assert response["status"] == "success"
    assert response["retrieval_path"] == "graph_only"
    assert response["found"] == 1
    assert query_rows_runner.await_count == 0


@pytest.mark.asyncio
async def test_run_recall_single_call_vector_failure_uses_graph_only_fallback_rows(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    async def _fake_policy(*_args, **_kwargs):
        mode = str(_kwargs.get("retrieval_mode") or "")
        if mode == "hybrid":
            return types.SimpleNamespace(
                retrieval_path="vector_only",
                retrieval_reason="graph_insufficient",
                graph_backend="fake",
                graph_hit_count=0,
                graph_confidence_score=0.1,
                graph_confidence_level="low",
                retrieval_plan_schema_id="schema-id",
                retrieval_plan={"schema": "omni.link_graph.retrieval_plan.v1"},
                graph_rows=(),
                graph_only_empty=False,
            )
        return types.SimpleNamespace(
            retrieval_path="graph_only",
            retrieval_reason="graph_only_requested",
            graph_backend="fake",
            graph_hit_count=1,
            graph_confidence_score=0.93,
            graph_confidence_level="high",
            retrieval_plan_schema_id="schema-id",
            retrieval_plan={"schema": "omni.link_graph.retrieval_plan.v1"},
            graph_rows=(
                {
                    "content": "graph fallback",
                    "source": "docs/fallback.md",
                    "score": 0.92,
                    "title": "",
                    "section": "",
                },
            ),
            graph_only_empty=False,
        )

    monkeypatch.setitem(
        sys.modules,
        "omni.rag.link_graph",
        types.SimpleNamespace(evaluate_link_graph_recall_policy=_fake_policy),
    )

    query_rows_runner = AsyncMock(side_effect=RuntimeError("Embedding timed out after 5s"))
    response = await run_recall_single_call(
        vector_store=_VectorStore(),
        query="architecture",
        keywords=[],
        collection="knowledge_chunks",
        limit=5,
        fetch_limit=5,
        min_score=0.0,
        preview=False,
        snippet_chars=150,
        retrieval_mode="hybrid",
        postprocess_rows=_identity_postprocess,
        query_rows_runner=query_rows_runner,
    )

    assert response["status"] == "success"
    assert response["retrieval_path"] == "graph_only"
    assert response["retrieval_reason"] == "graph_only_requested"
    assert response["found"] == 1


@pytest.mark.asyncio
async def test_run_recall_single_call_vector_failure_returns_empty_when_graph_unavailable(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    async def _fake_policy(*_args, **_kwargs):
        mode = str(_kwargs.get("retrieval_mode") or "")
        if mode == "hybrid":
            return types.SimpleNamespace(
                retrieval_path="vector_only",
                retrieval_reason="graph_insufficient",
                graph_backend="fake",
                graph_hit_count=0,
                graph_confidence_score=0.0,
                graph_confidence_level="none",
                retrieval_plan_schema_id="schema-id",
                retrieval_plan={"schema": "omni.link_graph.retrieval_plan.v1"},
                graph_rows=(),
                graph_only_empty=False,
            )
        raise RuntimeError("backend unavailable")

    monkeypatch.setitem(
        sys.modules,
        "omni.rag.link_graph",
        types.SimpleNamespace(evaluate_link_graph_recall_policy=_fake_policy),
    )

    query_rows_runner = AsyncMock(side_effect=RuntimeError("Embedding timed out after 5s"))
    response = await run_recall_single_call(
        vector_store=_VectorStore(),
        query="architecture",
        keywords=[],
        collection="knowledge_chunks",
        limit=5,
        fetch_limit=5,
        min_score=0.0,
        preview=False,
        snippet_chars=150,
        retrieval_mode="hybrid",
        postprocess_rows=_identity_postprocess,
        query_rows_runner=query_rows_runner,
    )

    assert response["status"] == "success"
    assert response["retrieval_path"] == "vector_only"
    assert response["retrieval_reason"] == "vector_error_fallback_empty"
    assert response["found"] == 0
    assert response["results"] == []


@pytest.mark.asyncio
async def test_run_recall_single_call_vector_only_strict_disables_graph_policy_and_fallback(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    policy_calls = {"count": 0}

    async def _fake_policy(*_args, **_kwargs):
        policy_calls["count"] += 1
        raise AssertionError("graph policy should be disabled in strict vector_only mode")

    monkeypatch.setitem(
        sys.modules,
        "omni.rag.link_graph",
        types.SimpleNamespace(evaluate_link_graph_recall_policy=_fake_policy),
    )

    query_rows_runner = AsyncMock(side_effect=RuntimeError("Embedding timed out after 5s"))
    response = await run_recall_single_call(
        vector_store=_VectorStore(),
        query="architecture",
        keywords=[],
        collection="knowledge_chunks",
        limit=5,
        fetch_limit=5,
        min_score=0.0,
        preview=False,
        snippet_chars=150,
        retrieval_mode="vector_only",
        postprocess_rows=_identity_postprocess,
        query_rows_runner=query_rows_runner,
        allow_graph_policy=False,
        allow_graph_fallback_on_vector_error=False,
    )

    assert response["status"] == "success"
    assert response["retrieval_path"] == "vector_only"
    assert response["retrieval_reason"] == "vector_error_fallback_empty"
    assert response["found"] == 0
    assert response["results"] == []
    assert policy_calls["count"] == 0
