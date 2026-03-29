"""
Tests for code.code_search command (formerly code).

Tests cover:
- Unified code search command (AST + Vector + Grep)
- Graph-based search orchestration
- Result formatting

Usage:
    python -m pytest packages/python/core/tests/units/code/test_code_search.py -v
"""

from __future__ import annotations

import asyncio
import importlib
import sys
from pathlib import Path

import pytest
from xiuxian_foundation.api.decorators import normalize_tool_result


def _scripts_root() -> Path:
    return Path(__file__).parent.parent / "scripts"


def _ensure_skill_paths() -> None:
    path_text = str(_scripts_root())
    if path_text not in sys.path:
        sys.path.insert(0, path_text)


def _load_search_commands_module():
    _ensure_skill_paths()
    sys.modules.pop("search.commands", None)
    return importlib.import_module("search.commands")


def _load_search_graph_module():
    _ensure_skill_paths()
    sys.modules.pop("search.graph", None)
    return importlib.import_module("search.graph")


def _load_search_state_module():
    _ensure_skill_paths()
    sys.modules.pop("search.state", None)
    return importlib.import_module("search.state")


def _load_search_engines_module():
    _ensure_skill_paths()
    sys.modules.pop("search.nodes.engines", None)
    return importlib.import_module("search.nodes.engines")


def _load_search_nodes_package():
    _ensure_skill_paths()
    sys.modules.pop("search.nodes", None)
    return importlib.import_module("search.nodes")


def _load_search_formatter_module():
    _ensure_skill_paths()
    sys.modules.pop("search.nodes.formatter", None)
    return importlib.import_module("search.nodes.formatter")


def _load_search_classifier_direct_module():
    _ensure_skill_paths()
    sys.modules.pop("search.nodes.classifier", None)
    return importlib.import_module("search.nodes.classifier")


def _unwrap_command_text(result: object) -> str:
    normalized = normalize_tool_result(result)
    content = normalized["content"]
    first = content[0]
    text = first.get("text", "")
    assert isinstance(text, str)
    return text


class TestCodeSearchCommand:
    """Tests for the code_search command."""

    @pytest.fixture
    def code_search(self):
        """Load the retained code_search command directly."""
        return _load_search_commands_module().code_search

    def test_code_search_command_exists(self, code_search):
        """Test that code_search command remains importable."""
        assert callable(code_search)

    def test_code_search_is_async(self, code_search):
        """Test that code_search is an async function."""
        assert asyncio.iscoroutinefunction(code_search)

    @pytest.mark.asyncio
    async def test_code_search_returns_xml_format(self, code_search):
        """Test that code_search returns XML-formatted output."""
        result = await code_search("def test_function")
        text = _unwrap_command_text(result)

        assert "<" in text and ">" in text

    @pytest.mark.asyncio
    async def test_code_search_class_query(self, code_search):
        """Test code_search with class query."""
        result = await code_search("class TestClass")
        assert len(_unwrap_command_text(result)) > 0

    @pytest.mark.asyncio
    async def test_code_search_function_query(self, code_search):
        """Test code_search with function query."""
        result = await code_search("def hello_world")
        assert len(_unwrap_command_text(result)) > 0

    @pytest.mark.asyncio
    async def test_code_search_with_session_id(self, code_search):
        """Test code_search with custom session_id."""
        result = await code_search("def test", session_id="test_session_123")
        assert len(_unwrap_command_text(result)) > 0

    @pytest.mark.asyncio
    async def test_code_search_empty_query(self, code_search):
        """Test code_search with empty query."""
        result = await code_search("")
        assert len(_unwrap_command_text(result)) > 0

    def test_code_search_is_plain_callable_surface(self, code_search):
        """Test that code_search stays a plain callable without decorator metadata."""
        assert not hasattr(code_search, "_command_metadata")


class TestSearchEngines:
    """Tests for search engine wrappers."""

    def test_run_ast_search_import(self):
        """Test that AST search engine can be imported."""
        run_ast_search = _load_search_engines_module().run_ast_search

        assert callable(run_ast_search)

    def test_run_grep_search_import(self):
        """Test that grep search engine can be imported."""
        run_grep_search = _load_search_engines_module().run_grep_search

        assert callable(run_grep_search)

    def test_run_vector_search_import(self):
        """Test that vector search engine can be imported."""
        run_vector_search = _load_search_engines_module().run_vector_search

        assert callable(run_vector_search)

    def test_extract_ast_pattern_class(self):
        """Test AST pattern extraction for class."""
        extract_ast_pattern = _load_search_engines_module().extract_ast_pattern

        result = extract_ast_pattern("Find the class User")
        assert result == "class User"

    def test_extract_ast_pattern_find_class(self):
        """Test AST pattern extraction for find-class query."""
        extract_ast_pattern = _load_search_engines_module().extract_ast_pattern

        result = extract_ast_pattern("Find class User")
        assert "class" in result and "User" in result

    def test_extract_ast_pattern_fallback(self):
        """Test AST pattern fallback for simple patterns."""
        extract_ast_pattern = _load_search_engines_module().extract_ast_pattern

        # Simple patterns like "def hello" should be returned as-is
        result = extract_ast_pattern("def hello")
        assert "def" in result or "hello" in result


class TestSearchGraph:
    """Tests for search graph components."""

    def test_search_graph_state_import(self):
        """Test that SearchGraphState can be imported."""
        SearchGraphState = _load_search_state_module().SearchGraphState

        assert SearchGraphState is not None

    def test_search_graph_state_creation(self):
        """Test SearchGraphState creation."""
        SearchGraphState = _load_search_state_module().SearchGraphState

        state = SearchGraphState(query="test query")
        assert state["query"] == "test query"

    def test_create_search_graph(self):
        """Test that search graph can be created."""
        create_search_graph = _load_search_graph_module().create_search_graph

        graph = create_search_graph()
        assert graph is not None


class TestSearchNodes:
    """Tests for search graph nodes."""

    def test_node_run_ast_search(self):
        """Test AST search node."""
        node_run_ast_search = _load_search_engines_module().node_run_ast_search
        SearchGraphState = _load_search_state_module().SearchGraphState

        state = SearchGraphState(query="class Test")
        result = node_run_ast_search(state)
        assert "raw_results" in result
        assert isinstance(result["raw_results"], list)

    def test_node_run_grep_search(self):
        """Test grep search node."""
        node_run_grep_search = _load_search_engines_module().node_run_grep_search
        SearchGraphState = _load_search_state_module().SearchGraphState

        state = SearchGraphState(query="def test")
        result = node_run_grep_search(state)
        assert "raw_results" in result

    def test_node_run_vector_search(self):
        """Test vector search node."""
        node_run_vector_search = _load_search_engines_module().node_run_vector_search
        SearchGraphState = _load_search_state_module().SearchGraphState

        state = SearchGraphState(query="test query")
        result = node_run_vector_search(state)
        assert "raw_results" in result


class TestSearchClassifier:
    """Tests for search classifier."""

    def test_classifier_import(self):
        """Test that classifier can be imported."""
        classifier = _load_search_classifier_direct_module()

        assert classifier is not None


class TestSearchFormatter:
    """Tests for search result formatter."""

    def test_formatter_import(self):
        """Test that formatter can be imported."""
        formatter = _load_search_formatter_module()

        assert formatter is not None
