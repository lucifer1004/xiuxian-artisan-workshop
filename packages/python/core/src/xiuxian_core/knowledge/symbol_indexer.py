"""Removal surface for Python-side symbol indexing."""

from __future__ import annotations

from typing import Any


_SYMBOL_INDEXER_REMOVAL_MESSAGE = (
    "Python symbol indexing has been removed. Use Rust/Wendao over Arrow "
    "Flight for symbol indexing."
)


class Symbol:
    def __init__(self, *args: Any, **kwargs: Any) -> None:
        _ = (args, kwargs)
        raise RuntimeError(_SYMBOL_INDEXER_REMOVAL_MESSAGE)


class SymbolIndex:
    def __init__(self, *args: Any, **kwargs: Any) -> None:
        _ = (args, kwargs)
        raise RuntimeError(_SYMBOL_INDEXER_REMOVAL_MESSAGE)


class SymbolIndexer:
    def __init__(self, *args: Any, **kwargs: Any) -> None:
        _ = (args, kwargs)
        raise RuntimeError(_SYMBOL_INDEXER_REMOVAL_MESSAGE)


def build_symbol_index(*args: Any, **kwargs: Any) -> SymbolIndexer:
    _ = (args, kwargs)
    raise RuntimeError(_SYMBOL_INDEXER_REMOVAL_MESSAGE)


__all__ = [
    "Symbol",
    "SymbolIndex",
    "SymbolIndexer",
    "build_symbol_index",
]
