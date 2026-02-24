from __future__ import annotations

import importlib
from pathlib import Path

import pytest


def _skill_root() -> Path:
    return Path(__file__).resolve().parents[1]


@pytest.mark.asyncio
async def test_search_memory_enables_semantic_cache(monkeypatch) -> None:
    monkeypatch.syspath_prepend(str(_skill_root()))
    memory_module = importlib.import_module("scripts.memory")

    observed: dict[str, object] = {}

    async def _fake_run_recall_semantic_rows(
        *,
        vector_store,
        query,
        collection,
        fetch_limit,
        use_cache,
    ):
        observed["vector_store"] = vector_store
        observed["query"] = query
        observed["collection"] = collection
        observed["fetch_limit"] = fetch_limit
        observed["use_cache"] = use_cache
        return []

    marker_store = object()
    monkeypatch.setattr(memory_module.vector_service, "get_vector_store", lambda: marker_store)
    monkeypatch.setattr(memory_module, "run_recall_semantic_rows", _fake_run_recall_semantic_rows)

    result = await memory_module.search_memory("cache probe", limit=3)
    if isinstance(result, dict) and "content" in result:
        text_items = result.get("content") or []
        assert isinstance(text_items, list) and text_items
        assert text_items[0].get("text") == "No matching memories found."
    else:
        assert result == "No matching memories found."
    assert observed["vector_store"] is marker_store
    assert observed["query"] == "cache probe"
    assert observed["collection"] == memory_module.DEFAULT_TABLE
    assert observed["fetch_limit"] == 3
    assert observed["use_cache"] is True
