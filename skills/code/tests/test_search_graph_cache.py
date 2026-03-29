"""Tests for cached native workflow compilation in code search graph."""

from __future__ import annotations

import importlib
import sys
from pathlib import Path

import pytest


def _scripts_root() -> Path:
    return Path(__file__).parent.parent / "scripts"


def _ensure_skill_paths() -> None:
    path_text = str(_scripts_root())
    if path_text not in sys.path:
        sys.path.insert(0, path_text)


def _load_search_graph_module():
    _ensure_skill_paths()
    sys.modules.pop("search.graph", None)
    return importlib.import_module("search.graph")


@pytest.fixture
def search_graph_module():
    """Load search graph module directly from retained scripts surface."""
    return _load_search_graph_module()


def test_get_compiled_search_graph_reuses_instance(search_graph_module) -> None:
    """The compiled graph should be created once per process."""
    search_graph = search_graph_module
    original_graph = search_graph._search_graph
    original_compiled = search_graph._compiled_search_graph
    try:
        search_graph._search_graph = None
        search_graph._compiled_search_graph = None

        compiled_first = search_graph.get_compiled_search_graph()
        compiled_second = search_graph.get_compiled_search_graph()

        assert compiled_first is compiled_second
    finally:
        search_graph._search_graph = original_graph
        search_graph._compiled_search_graph = original_compiled
