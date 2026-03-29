"""Removal tests for legacy Python vector store package surfaces."""

from __future__ import annotations

import importlib
import importlib.util


def test_vector_store_modules_removed() -> None:
    importlib.invalidate_caches()
    assert importlib.util.find_spec("xiuxian_foundation.services.vector.store") is None
    assert importlib.util.find_spec("xiuxian_foundation.services.vector.knowledge") is None
    assert importlib.util.find_spec("xiuxian_foundation.services.vector.hybrid") is None
    assert importlib.util.find_spec("xiuxian_foundation.services.vector.crud") is None


def test_vector_package_no_longer_exports_local_store_symbols() -> None:
    from xiuxian_foundation.services import vector

    assert not hasattr(vector, "VectorStoreClient")
    assert not hasattr(vector, "get_vector_store")
    assert not hasattr(vector, "search_knowledge")
    assert not hasattr(vector, "add_knowledge")
    assert not hasattr(vector, "evict_all_vector_stores")
    assert not hasattr(vector, "evict_knowledge_store_after_use")
