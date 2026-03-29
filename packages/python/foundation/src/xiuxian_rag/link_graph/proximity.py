"""LinkGraph proximity boost helpers for Python-side RAG post-processing."""

from __future__ import annotations

import asyncio
import logging
from pathlib import Path
from typing import Any

from xiuxian_foundation.config.settings import get_setting

logger = logging.getLogger(__name__)

DEFAULT_LINK_PROXIMITY_BOOST = 0.2
DEFAULT_TAG_PROXIMITY_BOOST = 0.1
DEFAULT_MAX_HOPS = 1
DEFAULT_TIMEOUT_SECONDS = 0.25

_stem_cache: dict[str, str] = {}


def _source_to_stem(source: str) -> str:
    cached = _stem_cache.get(source)
    if cached is not None:
        return cached
    stem = Path(source).stem if source else ""
    _stem_cache[source] = stem
    return stem


async def _load_backend(
    notebook_dir: str | Path | None,
) -> Any | None:
    del notebook_dir
    return None


async def apply_link_graph_proximity_boost(
    results: list[dict[str, Any]],
    query: str,
    *,
    backend: Any | None = None,
    notebook_dir: str | Path | None = None,
    link_boost: float = DEFAULT_LINK_PROXIMITY_BOOST,
    tag_boost: float = DEFAULT_TAG_PROXIMITY_BOOST,
    max_hops: int = DEFAULT_MAX_HOPS,
    fusion_scale: float | None = None,
) -> list[dict[str, Any]]:
    """Boost recall results when their sources are linked or tag-related."""
    if len(results) < 2:
        return results

    take_recent_timeout = _load_recent_timeout_checker()
    if take_recent_timeout and take_recent_timeout(query):
        return results

    injected_backend = backend is not None
    backend = backend or await _load_backend(notebook_dir)
    if backend is None:
        return results

    effective_link_boost = link_boost * (fusion_scale if fusion_scale is not None else 1.0)
    effective_tag_boost = tag_boost * (fusion_scale if fusion_scale is not None else 1.0)
    timeout_seconds = float(
        get_setting("link_graph.proximity.timeout_seconds", DEFAULT_TIMEOUT_SECONDS)
    )
    if timeout_seconds <= 0:
        timeout_seconds = DEFAULT_TIMEOUT_SECONDS

    stems = [_source_to_stem(str(row.get("source", ""))) for row in results]
    linked_by_stem: dict[str, set[str]] = {}
    tags_by_stem: dict[str, set[str]] = {}

    try:
        for stem in stems:
            if not stem or stem in linked_by_stem:
                continue
            if injected_backend:
                neighbors = await backend.neighbors(stem, hops=max_hops)
                metadata = await backend.metadata(stem)
            else:
                neighbors = await asyncio.wait_for(
                    backend.neighbors(stem, hops=max_hops),
                    timeout=timeout_seconds,
                )
                metadata = await asyncio.wait_for(
                    backend.metadata(stem),
                    timeout=timeout_seconds,
                )
            linked_by_stem[stem] = {neighbor.stem for neighbor in neighbors}
            tags_by_stem[stem] = set(getattr(metadata, "tags", []) or [])
    except TimeoutError:
        logger.debug("LinkGraph proximity boost timed out for query=%s", query)
        return results
    except Exception as exc:
        logger.debug("LinkGraph proximity boost skipped for query=%s: %s", query, exc)
        return results

    for index, left in enumerate(results):
        left_stem = stems[index]
        if not left_stem:
            continue
        for right_index in range(index + 1, len(results)):
            right = results[right_index]
            right_stem = stems[right_index]
            if not right_stem:
                continue

            boosted = False
            if right_stem in linked_by_stem.get(
                left_stem, set()
            ) or left_stem in linked_by_stem.get(right_stem, set()):
                left["score"] = float(left.get("score", 0.0)) + effective_link_boost
                right["score"] = float(right.get("score", 0.0)) + effective_link_boost
                boosted = True

            if tags_by_stem.get(left_stem, set()) & tags_by_stem.get(right_stem, set()):
                left["score"] = float(left.get("score", 0.0)) + effective_tag_boost
                right["score"] = float(right.get("score", 0.0)) + effective_tag_boost
                boosted = True

            if boosted:
                left["final_score"] = float(left.get("final_score", left["score"]))
                right["final_score"] = float(right.get("final_score", right["score"]))

    results.sort(key=lambda row: float(row.get("score", 0.0)), reverse=True)
    return results


def _load_recent_timeout_checker() -> Any | None:
    try:
        from .policy import take_recent_graph_search_timeout

        return take_recent_graph_search_timeout
    except Exception:
        return None


__all__ = [
    "DEFAULT_LINK_PROXIMITY_BOOST",
    "DEFAULT_MAX_HOPS",
    "DEFAULT_TAG_PROXIMITY_BOOST",
    "apply_link_graph_proximity_boost",
]
