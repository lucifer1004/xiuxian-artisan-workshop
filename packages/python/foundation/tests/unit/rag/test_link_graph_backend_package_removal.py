"""Removal tests for legacy LinkGraph backend and factory surfaces."""

from __future__ import annotations

import importlib
import importlib.util


def test_link_graph_backend_modules_removed() -> None:
    importlib.invalidate_caches()
    assert importlib.util.find_spec("xiuxian_rag.link_graph.backend") is None
    assert importlib.util.find_spec("xiuxian_rag.link_graph.factory") is None
    assert importlib.util.find_spec("xiuxian_rag.link_graph.wendao_backend") is None


def test_link_graph_package_no_longer_exports_backend_symbols() -> None:
    import xiuxian_rag.link_graph as link_graph

    assert not hasattr(link_graph, "LinkGraphBackend")
    assert not hasattr(link_graph, "WendaoLinkGraphBackend")
    assert not hasattr(link_graph, "get_link_graph_backend")
    assert not hasattr(link_graph, "reset_link_graph_backend_cache")
