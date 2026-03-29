"""Removal checks for Python-side fusion router/KG hooks."""

from __future__ import annotations

import importlib

import pytest


def test_graph_enrichment_module_removed() -> None:
    with pytest.raises(ModuleNotFoundError):
        importlib.import_module("xiuxian_rag.fusion.graph_enrichment")


def test_kg_rerank_module_removed() -> None:
    with pytest.raises(ModuleNotFoundError):
        importlib.import_module("xiuxian_rag.fusion.kg_rerank")


def test_fusion_package_no_longer_exports_router_hooks() -> None:
    import xiuxian_rag.fusion as fusion

    assert "enrich_skill_graph_from_link_graph" not in fusion.__all__
    assert "register_skill_entities" not in fusion.__all__
    assert "apply_kg_rerank" not in fusion.__all__


def test_rag_package_no_longer_exports_router_hooks() -> None:
    import xiuxian_rag as rag

    assert "enrich_skill_graph_from_link_graph" not in rag.__all__
    assert "register_skill_entities" not in rag.__all__
