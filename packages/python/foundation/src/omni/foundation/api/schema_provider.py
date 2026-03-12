"""
Centralized schema provider that loads JSON schemas from Rust binary bindings.
Follows CyberXiuXian Artisan Studio 2026 standards for self-contained resources.
"""

from __future__ import annotations

import json
from functools import lru_cache
from typing import Any


_OMNI_CORE_RS_SCHEMA_MAP: dict[str, str] = {
    # Rust omni_core_rs exposes type-based schema retrieval.
    "omni.vector.hybrid.v1": "HybridSearchResult",
    "omni.vector.search.v1": "VectorSearchResult",
    "omni.vector.tool_search.v1": "ToolSearchResult",
}


@lru_cache(maxsize=None)
def get_schema(name: str) -> dict[str, Any]:
    """
    Load a schema by name from the Rust backend.

    Args:
        name: The canonical schema identifier (e.g., 'omni.link_graph.record.v1')

    Returns:
        The parsed JSON schema as a dictionary.

    Raises:
        ImportError: If the Rust backend is not available.
        ValueError: If the schema name is unknown.
    """
    # Preferred backend: xiuxian_wendao schema registry (canonical id -> JSON schema)
    try:
        from _xiuxian_wendao import get_schema as rust_get_schema
    except ImportError:
        rust_get_schema = None
    if rust_get_schema is not None:
        try:
            return json.loads(rust_get_schema(name))
        except ValueError as e:
            raise ValueError(f"Unknown schema identifier: {name}") from e
        except Exception as e:
            raise RuntimeError(f"Failed to load schema '{name}' from Rust binding: {e}") from e

    # Canonical backend via omni_core_rs (schema-id based registry).
    try:
        import omni_core_rs

        if hasattr(omni_core_rs, "py_get_named_schema_json"):
            return json.loads(omni_core_rs.py_get_named_schema_json(name))
    except Exception:
        # Fall through to the legacy type-mapped fallback below.
        pass

    # Secondary backend: omni_core_rs type registry (subset mapping only)
    mapped_type = _OMNI_CORE_RS_SCHEMA_MAP.get(name)
    if mapped_type is not None:
        try:
            import omni_core_rs

            return json.loads(omni_core_rs.py_get_schema_json(mapped_type))
        except Exception as e:
            raise RuntimeError(
                f"Failed to load schema '{name}' from omni_core_rs type '{mapped_type}': {e}"
            ) from e

    raise ImportError(
        "No Rust schema binding available for "
        f"'{name}'. Install `_xiuxian_wendao` or expose this schema via `omni_core_rs`."
    )


def get_schema_id(name: str) -> str:
    """Return the $id field from a schema."""
    schema = get_schema(name)
    return str(schema.get("$id", "")).strip()
