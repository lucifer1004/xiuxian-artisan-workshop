"""Vector (semantic/recall) search over knowledge store."""

from __future__ import annotations

from typing import TYPE_CHECKING, Any

from omni.foundation.config.logging import get_logger
from omni.foundation.services.vector import get_vector_store
from omni.rag.retrieval import run_recall_single_call

logger = get_logger("skill.knowledge.search.vector")

if TYPE_CHECKING:
    from omni.foundation.config.paths import ConfigPaths


async def _identity_postprocess(
    rows: list[dict[str, Any]],
    _query: str,
    _limit: int,
    _min_score: float,
    _preview: bool,
    _snippet_chars: int,
    _apply_fusion_boost: bool,
) -> list[dict[str, Any]]:
    return list(rows)


async def run_vector_search(
    query: str,
    limit: int = 10,
    collection: str = "knowledge_chunks",
    paths: ConfigPaths | None = None,
) -> dict[str, Any]:
    """Run vector/semantic search via common retrieval executor."""
    del paths
    vector_store = get_vector_store()
    active_store = vector_store.get_store_for_collection(collection)
    if not active_store:
        return {
            "success": True,
            "query": query,
            "status": "unavailable",
            "found": 0,
            "retrieval_mode": "vector_only",
            "retrieval_path": "vector_only",
            "retrieval_reason": "vector_store_unavailable",
            "results": [],
        }

    response = await run_recall_single_call(
        vector_store=vector_store,
        query=query,
        keywords=[],
        collection=collection,
        limit=limit,
        fetch_limit=limit,
        min_score=0.0,
        preview=False,
        snippet_chars=150,
        retrieval_mode="vector_only",
        postprocess_rows=_identity_postprocess,
        debug_log=logger.debug,
        warning_log=logger.warning,
        allow_graph_policy=False,
        allow_graph_fallback_on_vector_error=False,
    )
    return {"success": True, **response}


__all__ = ["run_vector_search"]
