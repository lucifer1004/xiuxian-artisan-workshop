"""Bridge configuration: boost constants and KnowledgeGraph Valkey scope resolution."""

from __future__ import annotations

import logging
from typing import Any

logger = logging.getLogger("omni.rag.dual_core")

# ---------------------------------------------------------------------------
# Constants
# ---------------------------------------------------------------------------

# Score boost for recall results whose source docs are link-graph linked.
LINK_GRAPH_LINK_PROXIMITY_BOOST = 0.12

# Score boost for recall results sharing link-graph metadata tags.
LINK_GRAPH_TAG_PROXIMITY_BOOST = 0.08

# Score boost for router tools connected via graph entity relations.
LINK_GRAPH_ENTITY_BOOST = 0.10

# Maximum link-graph hops to consider for proximity.
MAX_LINK_GRAPH_HOPS = 2

# Timeout (seconds) for link-graph neighbor/tag fetch.
LINK_GRAPH_PROXIMITY_TIMEOUT = 5

# Max stems to fetch link-graph context for (top by result order).
LINK_GRAPH_MAX_STEMS = 8

# TTL (seconds) for in-memory link-graph stem cache; 0 = disabled.
LINK_GRAPH_STEM_CACHE_TTL_SEC = 60

# Score boost per unit of KG relevance (tool relevance from multi-hop graph walk)
KG_QUERY_RERANK_SCALE = 0.08

# Max results to consider from KG relevance query
KG_QUERY_LIMIT = 15


# ---------------------------------------------------------------------------
# Graph scope resolver
# ---------------------------------------------------------------------------


def _resolve_graph_scope_key(scope_key: str | None = None) -> str:
    """Resolve the stable KnowledgeGraph scope key used for Valkey snapshots."""
    if scope_key is not None and str(scope_key).strip():
        return str(scope_key)
    from omni.foundation.config.database import get_database_path

    # Reuse knowledge DB identity as stable graph scope namespace.
    return get_database_path("knowledge")


# ---------------------------------------------------------------------------
# KG load / save (Valkey-backed)
# ---------------------------------------------------------------------------


def _load_kg(
    *,
    scope_key: str | None = None,
) -> Any | None:
    """Load KnowledgeGraph from a Valkey snapshot.

    Uses Rust-side cache (``load_kg_from_valkey_cached``) to avoid repeated
    backend reads during recall. Cache is invalidated on save.

    Returns:
        Loaded PyKnowledgeGraph, or None if import fails.
    """
    try:
        from omni_core_rs import load_kg_from_valkey_cached
    except ImportError:
        return None

    resolved_scope = _resolve_graph_scope_key(scope_key)
    try:
        result = load_kg_from_valkey_cached(resolved_scope)
    except Exception as exc:
        logger.debug("KG load from Valkey skipped (%s): %s", resolved_scope, exc)
        return None
    if result is None:
        return None
    logger.debug("KG loaded from Valkey (cached): %s", resolved_scope)
    return result


def _save_kg(
    kg: Any,
    *,
    scope_key: str | None = None,
) -> None:
    """Save KnowledgeGraph to a Valkey snapshot.

    Rust ``save_to_valkey`` invalidates the KG cache for this scope automatically.
    """
    resolved_scope = _resolve_graph_scope_key(scope_key)
    kg.save_to_valkey(resolved_scope)
    logger.debug("KG saved to Valkey: %s", resolved_scope)
