"""
Dual-Core Knowledge Fusion Engine.

Bridges Core 1 (LinkGraph) and Core 2 (LanceDB vector) into unified search.

Sub-modules by bridge:
- link_graph_proximity: Bridge 1 — LinkGraph proximity boost for recall
- fusion_weights: Dynamic weight selection based on Python query heuristics
"""

from ._config import (
    KG_QUERY_LIMIT,
    LINK_GRAPH_LINK_PROXIMITY_BOOST,
    LINK_GRAPH_TAG_PROXIMITY_BOOST,
    MAX_LINK_GRAPH_HOPS,
)
from .fusion_weights import FusionWeights, compute_fusion_weights
from .link_graph_proximity import link_graph_proximity_boost

__all__ = [
    "KG_QUERY_LIMIT",
    "LINK_GRAPH_LINK_PROXIMITY_BOOST",
    "LINK_GRAPH_TAG_PROXIMITY_BOOST",
    "MAX_LINK_GRAPH_HOPS",
    "FusionWeights",
    "compute_fusion_weights",
    "link_graph_proximity_boost",
]
