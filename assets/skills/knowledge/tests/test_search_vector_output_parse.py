"""Unit tests for search.vector common retrieval adapter."""

from __future__ import annotations

import pytest
from search import vector as vector_mode


@pytest.mark.asyncio
async def test_run_vector_search_uses_common_single_call(monkeypatch: pytest.MonkeyPatch) -> None:
    class _VectorStore:
        def get_store_for_collection(self, collection: str) -> object:
            assert collection == "knowledge_chunks"
            return object()

    async def _fake_single_call(**kwargs):
        assert kwargs["retrieval_mode"] == "vector_only"
        assert kwargs["limit"] == 3
        assert kwargs["fetch_limit"] == 3
        assert kwargs["allow_graph_policy"] is False
        assert kwargs["allow_graph_fallback_on_vector_error"] is False
        return {
            "query": "architecture",
            "status": "success",
            "found": 1,
            "retrieval_mode": "vector_only",
            "retrieval_path": "vector_only",
            "retrieval_reason": "vector_default",
            "results": [{"content": "result"}],
        }

    monkeypatch.setattr(vector_mode, "get_vector_store", lambda: _VectorStore())
    monkeypatch.setattr(vector_mode, "run_recall_single_call", _fake_single_call)

    out = await vector_mode.run_vector_search("architecture", limit=3)
    assert out["success"] is True
    assert out["status"] == "success"
    assert out["retrieval_mode"] == "vector_only"
    assert out["retrieval_path"] == "vector_only"
    assert out["found"] == 1
    assert isinstance(out["results"], list)


@pytest.mark.asyncio
async def test_run_vector_search_returns_unavailable_when_store_missing(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    class _VectorStore:
        def get_store_for_collection(self, collection: str):
            assert collection == "knowledge_chunks"
            return None

    monkeypatch.setattr(vector_mode, "get_vector_store", lambda: _VectorStore())
    out = await vector_mode.run_vector_search("architecture", limit=3)
    assert out["success"] is True
    assert out["status"] == "unavailable"
    assert out["retrieval_reason"] == "vector_store_unavailable"
    assert out["results"] == []
