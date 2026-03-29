"""Removal surface for Python-side Librarian."""

from __future__ import annotations

from typing import Any


_LIBRARIAN_REMOVAL_MESSAGE = (
    "Python Librarian has been removed. Use Rust/Wendao over Arrow Flight for "
    "knowledge ingestion, retrieval, and analysis."
)


def _raise_removed() -> None:
    raise RuntimeError(_LIBRARIAN_REMOVAL_MESSAGE)


class ChunkMode:
    TEXT = "text"
    AST = "ast"


class Librarian:
    def __init__(self, *args: Any, **kwargs: Any) -> None:
        _ = (args, kwargs)
        _raise_removed()


class KnowledgeStorage:
    def __init__(self, *args: Any, **kwargs: Any) -> None:
        _ = (args, kwargs)
        _raise_removed()


__all__ = ["ChunkMode", "KnowledgeStorage", "Librarian"]
