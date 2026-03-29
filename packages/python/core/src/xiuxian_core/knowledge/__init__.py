"""Removal surface for Python-side knowledge subsystem.

Rust/Wendao owns knowledge ingestion, retrieval, graph analysis, and query
execution. Python no longer hosts a local knowledge subsystem.
"""

from __future__ import annotations

from typing import Any


_KNOWLEDGE_REMOVAL_MESSAGE = (
    "Python knowledge subsystem has been removed. Use Rust/Wendao over Arrow "
    "Flight for knowledge ingestion, retrieval, and analysis."
)


def _raise_removed() -> None:
    raise RuntimeError(_KNOWLEDGE_REMOVAL_MESSAGE)


class KnowledgeConfig:
    def __init__(self, *args: Any, **kwargs: Any) -> None:
        _ = (args, kwargs)
        _raise_removed()


def get_knowledge_config() -> KnowledgeConfig:
    _raise_removed()


def reset_config() -> None:
    return None


class Librarian:
    def __init__(self, *args: Any, **kwargs: Any) -> None:
        _ = (args, kwargs)
        _raise_removed()


class ChunkMode:
    TEXT = "text"
    AST = "ast"


__all__ = [
    "ChunkMode",
    "KnowledgeConfig",
    "Librarian",
    "get_knowledge_config",
    "reset_config",
]
