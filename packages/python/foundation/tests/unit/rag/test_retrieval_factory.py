"""Tests for the removed Python retrieval backend factory."""

from __future__ import annotations

import pytest

from xiuxian_rag.retrieval.factory import create_retrieval_backend


@pytest.mark.parametrize("kind", ["lance", "hybrid", "lancedb", "vector", "unknown"])
def test_factory_rejects_removed_python_backends(kind: str) -> None:
    with pytest.raises(RuntimeError, match="Arrow Flight retrieval"):
        create_retrieval_backend(kind)
