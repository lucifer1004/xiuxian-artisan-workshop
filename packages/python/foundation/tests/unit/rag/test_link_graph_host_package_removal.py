"""Removal tests for legacy Python link-graph host helpers."""

from __future__ import annotations

import importlib
import importlib.util


def test_unified_knowledge_and_navigator_modules_removed() -> None:
    importlib.invalidate_caches()
    assert importlib.util.find_spec("xiuxian_rag.unified_knowledge") is None
    assert importlib.util.find_spec("xiuxian_rag.link_graph_navigator") is None


def test_rag_namespace_no_longer_exports_link_graph_hosts() -> None:
    import xiuxian_rag as rag

    assert not hasattr(rag, "NavigationConfig")
    assert not hasattr(rag, "LinkGraphNavigator")
    assert not hasattr(rag, "UnifiedEntity")
    assert not hasattr(rag, "UnifiedKnowledgeManager")
    assert not hasattr(rag, "get_link_graph_navigator")
    assert not hasattr(rag, "get_unified_manager")
