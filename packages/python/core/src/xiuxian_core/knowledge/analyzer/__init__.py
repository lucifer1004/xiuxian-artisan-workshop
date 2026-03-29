"""Removal surface for Python-side knowledge analytics."""

from __future__ import annotations

from typing import Any


_ANALYZER_REMOVAL_MESSAGE = (
    "Python knowledge analytics have been removed. Use Rust/Wendao over Arrow "
    "Flight for knowledge analysis."
)


def _raise_removed(*args: Any, **kwargs: Any) -> None:
    _ = (args, kwargs)
    raise RuntimeError(_ANALYZER_REMOVAL_MESSAGE)


def get_knowledge_dataframe(collection: str = "knowledge") -> None:
    _raise_removed(collection=collection)


def get_type_distribution(collection: str = "knowledge") -> dict[str, int]:
    _raise_removed(collection=collection)


def get_source_distribution(
    collection: str = "knowledge", limit: int | None = None
) -> dict[str, int]:
    _raise_removed(collection=collection, limit=limit)


def analyze_knowledge(collection: str = "knowledge", limit: int | None = None) -> dict[str, Any]:
    _raise_removed(collection=collection, limit=limit)


__all__ = [
    "analyze_knowledge",
    "get_knowledge_dataframe",
    "get_source_distribution",
    "get_type_distribution",
]
