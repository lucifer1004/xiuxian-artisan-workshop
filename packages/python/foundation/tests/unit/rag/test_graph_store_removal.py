"""Removal tests for the retired Python-side knowledge graph store surface."""

from __future__ import annotations

import xiuxian_foundation as foundation
import xiuxian_rag as rag


def test_rag_namespace_does_not_export_graph_store() -> None:
    assert not hasattr(rag, "KnowledgeGraphStore")
    assert not hasattr(rag, "get_graph_store")


def test_foundation_namespace_does_not_export_graph_store() -> None:
    assert not hasattr(foundation, "KnowledgeGraphStore")
    assert not hasattr(foundation, "get_graph_store")
