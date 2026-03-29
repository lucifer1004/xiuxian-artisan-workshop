"""Thin vector helper namespace."""

from __future__ import annotations

from .constants import (
    ERROR_BINDING_API_MISSING,
    ERROR_HYBRID_PAYLOAD_VALIDATION,
    ERROR_HYBRID_RUNTIME,
    ERROR_HYBRID_TABLE_NOT_FOUND,
    ERROR_PAYLOAD_VALIDATION,
    ERROR_REQUEST_VALIDATION,
    ERROR_RUNTIME,
    ERROR_TABLE_NOT_FOUND,
    MAX_SEARCH_RESULTS,
)
from .search import (
    SEARCH_EMBED_TIMEOUT,
    search_embed_timeout,
)
from .models import SearchResult

# Legacy names for callers that use leading-underscore names
_search_embed_timeout = search_embed_timeout

__all__ = [
    "ERROR_BINDING_API_MISSING",
    "ERROR_HYBRID_PAYLOAD_VALIDATION",
    "ERROR_HYBRID_RUNTIME",
    "ERROR_HYBRID_TABLE_NOT_FOUND",
    "ERROR_PAYLOAD_VALIDATION",
    "ERROR_REQUEST_VALIDATION",
    "ERROR_RUNTIME",
    "ERROR_TABLE_NOT_FOUND",
    "MAX_SEARCH_RESULTS",
    "SEARCH_EMBED_TIMEOUT",
    "SearchResult",
    "search_embed_timeout",
]
