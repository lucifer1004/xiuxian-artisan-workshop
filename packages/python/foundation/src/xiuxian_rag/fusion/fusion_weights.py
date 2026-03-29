"""Dynamic fusion weight selection based on lightweight Python query parsing.

Determines how much emphasis to place on graph/proximity vs vector/keyword
signals depending on query characteristics:

- Knowledge/docs-oriented queries → boost ZK proximity & graph rerank
- Code/tool-oriented queries → boost LanceDB vector precision
- Generic queries → balanced defaults
"""

from __future__ import annotations

import logging
from dataclasses import dataclass, field

logger = logging.getLogger("xiuxian_rag.dual_core.fusion")


@dataclass(frozen=True)
class FusionWeights:
    """Per-query weights controlling graph vs vector emphasis.

    Attributes:
        zk_proximity_scale: Multiplier for link-graph proximity boost.
        zk_entity_scale: Reserved graph-side weight for thin Python enhancement.
        kg_rerank_scale: Reserved graph-side rerank weight.
        vector_weight: Emphasis on Rust vector retrieval.
        keyword_weight: Emphasis on keyword routing.
        intent_action: Heuristic action derived from the query.
        intent_target: Heuristic target derived from the query.
    """

    zk_proximity_scale: float = 1.0
    zk_entity_scale: float = 1.0
    kg_rerank_scale: float = 1.0
    vector_weight: float = 1.0
    keyword_weight: float = 1.0
    intent_action: str | None = None
    intent_target: str | None = None
    intent_keywords: list[str] = field(default_factory=list)


# Default balanced weights
_BALANCED = FusionWeights()

# Target → graph emphasis profile
_ZK_HEAVY_TARGETS = {"knowledge", "docs"}
# Target → vector emphasis profile
_VECTOR_HEAVY_TARGETS = {"code", "database", "skill", "test"}
# Actions that benefit from graph context
_ZK_ACTIONS = {"search", "research"}
# Actions that need precise tool routing
_TOOL_ACTIONS = {"commit", "push", "pull", "merge", "rebase", "run", "test", "lint", "format"}

_STOPWORDS = {
    "the",
    "a",
    "an",
    "is",
    "are",
    "to",
    "for",
    "in",
    "on",
    "of",
    "help",
    "me",
    "my",
    "please",
    "want",
    "need",
    "with",
    "and",
    "or",
    "about",
    "find",
    "search",
}


def _extract_query_intent(query: str) -> tuple[str | None, str | None, list[str]]:
    tokens = [token.strip(".,:;!?()[]{}").lower() for token in query.split()]
    tokens = [token for token in tokens if token]
    keywords = [token for token in tokens if len(token) >= 2 and token not in _STOPWORDS]

    action = None
    for candidate in ("research", "search", "commit", "push", "pull", "merge", "run", "test"):
        if candidate in tokens:
            action = candidate
            break

    target = None
    if "knowledge" in tokens or "docs" in tokens:
        target = "knowledge" if "knowledge" in tokens else "docs"
    elif "codebase" in tokens or "code" in tokens or "function" in tokens:
        target = "code"
    elif "git" in tokens:
        target = "git"
    elif "database" in tokens or "sql" in tokens:
        target = "database"
    elif "skill" in tokens or "skills" in tokens:
        target = "skill"
    elif "test" in tokens or "tests" in tokens:
        target = "test"

    return action, target, keywords


def compute_fusion_weights(query: str) -> FusionWeights:
    """Compute dynamic fusion weights from a user query.

    Args:
        query: Raw user query string.

    Returns:
        FusionWeights with per-signal scaling.
    """
    if not query or not query.strip():
        return _BALANCED

    action, target, keywords = _extract_query_intent(query)

    # Start from balanced
    zk_prox = 1.0
    zk_ent = 1.0
    kg_rerank = 1.0
    vec_w = 1.0
    kw_w = 1.0

    # --- Target-based adjustments ---
    if target in _ZK_HEAVY_TARGETS:
        # Knowledge / docs queries: graph context is more valuable
        zk_prox = 1.5
        zk_ent = 1.4
        kg_rerank = 1.3
        vec_w = 0.9
    elif target in _VECTOR_HEAVY_TARGETS:
        # Code / tool queries: vector precision matters more
        zk_prox = 0.7
        zk_ent = 0.8
        kg_rerank = 0.9
        vec_w = 1.2
        kw_w = 1.3

    # --- Action-based refinements ---
    # Only apply action adjustments if no specific target was detected,
    # or if the target is in a compatible group. Target provides stronger
    # domain signal than action alone.
    has_specific_target = target is not None
    if action in _ZK_ACTIONS and target not in _VECTOR_HEAVY_TARGETS:
        # Search / research benefits from broader graph context
        zk_prox = max(zk_prox, 1.3)
        kg_rerank = max(kg_rerank, 1.2)
    elif action in _TOOL_ACTIONS:
        # Precise tool routing — favor keyword/vector exact match
        vec_w = max(vec_w, 1.1)
        kw_w = max(kw_w, 1.4)
        if not has_specific_target or target not in _ZK_HEAVY_TARGETS:
            zk_prox = min(zk_prox, 0.8)

    # --- Keyword density heuristic ---
    # Many keywords → broader query → ZK graph helps disambiguate
    if len(keywords) >= 4:
        kg_rerank *= 1.1
        zk_ent *= 1.1

    weights = FusionWeights(
        zk_proximity_scale=round(zk_prox, 2),
        zk_entity_scale=round(zk_ent, 2),
        kg_rerank_scale=round(kg_rerank, 2),
        vector_weight=round(vec_w, 2),
        keyword_weight=round(kw_w, 2),
        intent_action=action,
        intent_target=target,
        intent_keywords=keywords,
    )

    logger.debug(
        "Fusion weights computed: action=%s target=%s → zk_prox=%.2f kg_rerank=%.2f vec=%.2f kw=%.2f",
        action,
        target,
        weights.zk_proximity_scale,
        weights.kg_rerank_scale,
        weights.vector_weight,
        weights.keyword_weight,
    )

    return weights
