"""ZK-only search (link reasoning, no vector)."""

from __future__ import annotations

from typing import Any

from omni.foundation.config.logging import get_logger
from omni.foundation.config.paths import ConfigPaths
from omni.rag.zk_search import ZkReasoningSearcher, ZkSearchConfig

logger = get_logger("skill.knowledge.search.zk")

_zk_searcher: ZkReasoningSearcher | None = None


def _get_searcher(paths: ConfigPaths | None = None) -> ZkReasoningSearcher:
    """Get or create the ZK searcher instance."""
    global _zk_searcher
    if _zk_searcher is None:
        if paths is None:
            paths = ConfigPaths()
        _zk_searcher = ZkReasoningSearcher(
            notebook_dir=str(paths.project_root),
            config=ZkSearchConfig(max_iterations=3, max_notes_per_iteration=10),
        )
    return _zk_searcher


async def run_zk_search(
    query: str,
    max_results: int = 10,
    paths: ConfigPaths | None = None,
) -> dict[str, Any]:
    """Run ZK-only search; returns success, query, total, results, graph_stats."""
    if paths is None:
        paths = ConfigPaths()
    searcher = _get_searcher(paths)
    zk_results = await searcher.search(query, max_results=max_results)
    results = []
    for r in zk_results:
        note = getattr(r, "note", None)
        lead = (getattr(note, "lead", None) or "")[:200] if note else ""
        results.append(
            {
                "title": getattr(note, "title", "") if note else "",
                "id": getattr(note, "filename_stem", "") if note else "",
                "path": getattr(note, "path", "") if note else getattr(r, "path", ""),
                "score": getattr(r, "relevance_score", None) or getattr(r, "score", 0),
                "source": getattr(r, "source", ""),
                "distance": getattr(r, "distance", 0),
                "reasoning": getattr(r, "reasoning", ""),
                "lead": lead,
            }
        )
    graph_stats = {}
    if searcher.enhancer and getattr(searcher.enhancer, "graph", None):
        graph_stats = searcher.enhancer.get_graph_stats()
    return {
        "success": True,
        "query": query,
        "total": len(results),
        "results": results,
        "graph_stats": graph_stats,
    }


__all__ = ["run_zk_search"]
