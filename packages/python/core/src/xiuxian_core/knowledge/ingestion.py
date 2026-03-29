"""Removal surface for Python-side knowledge ingestion."""

from __future__ import annotations

from typing import Any


_INGESTION_REMOVAL_MESSAGE = (
    "Python knowledge ingestion has been removed. Use Rust/Wendao over Arrow "
    "Flight for ingestion and indexing."
)


class FileIngestor:
    def __init__(self, *args: Any, **kwargs: Any) -> None:
        _ = (args, kwargs)
        raise RuntimeError(_INGESTION_REMOVAL_MESSAGE)


__all__ = ["FileIngestor"]
