"""Legacy retrieval factory surface.

Python-owned local retrieval backends were removed. Retrieval execution now
belongs to the Rust runtime and Arrow Flight transport path.
"""

from __future__ import annotations

from typing import Any

from .interface import RetrievalBackend


def create_retrieval_backend(
    kind: str,
    *,
    vector_client: Any | None = None,
) -> RetrievalBackend:
    """Reject legacy local retrieval backend construction."""
    del vector_client
    raise RuntimeError(
        "Python local retrieval backends were removed; use Rust Arrow Flight retrieval "
        f"instead of create_retrieval_backend({kind!r})."
    )


__all__ = [
    "create_retrieval_backend",
]
