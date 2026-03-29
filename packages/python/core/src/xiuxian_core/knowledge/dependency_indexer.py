"""Removal surface for Python-side dependency indexing."""

from __future__ import annotations

from typing import Any


_DEPENDENCY_INDEXER_REMOVAL_MESSAGE = (
    "Python dependency indexing has been removed. Use Rust/Wendao over Arrow "
    "Flight for dependency and symbol indexing."
)


class DependencyIndexer:
    def __init__(self, *args: Any, **kwargs: Any) -> None:
        _ = (args, kwargs)
        raise RuntimeError(_DEPENDENCY_INDEXER_REMOVAL_MESSAGE)


__all__ = ["DependencyIndexer"]
