"""Bridge configuration constants for Python-side RAG post-processing."""

from __future__ import annotations

import logging
logger = logging.getLogger("xiuxian_rag.dual_core")

# ---------------------------------------------------------------------------
# Constants
# ---------------------------------------------------------------------------

# Score boost for recall results whose source docs are link-graph linked.
LINK_GRAPH_LINK_PROXIMITY_BOOST = 0.12

# Score boost for recall results sharing link-graph metadata tags.
LINK_GRAPH_TAG_PROXIMITY_BOOST = 0.08

# Maximum link-graph hops to consider for proximity.
MAX_LINK_GRAPH_HOPS = 2

# Timeout (seconds) for link-graph neighbor/tag fetch.
LINK_GRAPH_PROXIMITY_TIMEOUT = 5

# Max stems to fetch link-graph context for (top by result order).
LINK_GRAPH_MAX_STEMS = 8

# TTL (seconds) for in-memory link-graph stem cache; 0 = disabled.
LINK_GRAPH_STEM_CACHE_TTL_SEC = 60

# Max results to consider from post-process link-graph context.
KG_QUERY_LIMIT = 15
