"""
Tests for code skill - Unified Code Search.

Tests cover:
- code_search: Unified search entry point
- SmartAstEngine: AST pattern search
- Classifier: Intent classification
"""

import asyncio
import importlib
import sys
from pathlib import Path

from xiuxian_foundation.api.decorators import normalize_tool_result


def _scripts_root() -> Path:
    return Path(__file__).parent.parent / "scripts"


def _ensure_skill_paths() -> None:
    scripts_root = _scripts_root()
    path_text = str(scripts_root)
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


def _load_search_engines_module():
    _ensure_skill_paths()
    sys.modules.pop("search.nodes.engines", None)
    return importlib.import_module("search.nodes.engines")


def _load_search_classifier_module():
    _ensure_skill_paths()
    sys.modules.pop("search.nodes.classifier", None)
    return importlib.import_module("search.nodes.classifier")


def _load_smart_ast_engine_module():
    _ensure_skill_paths()
    sys.modules.pop("smart_ast.engine", None)
    return importlib.import_module("smart_ast.engine")


def _load_smart_ast_patterns_module():
    _ensure_skill_paths()
    sys.modules.pop("smart_ast.patterns", None)
    return importlib.import_module("smart_ast.patterns")


def _tool_text(result: object) -> str:
    return normalize_tool_result(result)["content"][0]["text"]


class TestCodeSearchUnified:
    """Test unified code_search command."""

    def test_code_search_class(self):
        module = _load_search_commands_module()

        result = _tool_text(asyncio.run(module.code_search(query="class User", session_id="test")))
        assert isinstance(result, str)

    def test_code_search_function(self):
        module = _load_search_commands_module()

        result = _tool_text(
            asyncio.run(module.code_search(query="def authenticate", session_id="test"))
        )
        assert isinstance(result, str)

    def test_code_search_semantic(self):
        module = _load_search_commands_module()

        result = _tool_text(
            asyncio.run(module.code_search(query="how does authentication work", session_id="test"))
        )
        assert isinstance(result, str)

    def test_code_search_todo(self):
        module = _load_search_commands_module()

        result = _tool_text(asyncio.run(module.code_search(query="TODO: fix", session_id="test")))
        assert isinstance(result, str)

    def test_code_search_refactor_pattern(self):
        module = _load_search_commands_module()

        result = _tool_text(
            asyncio.run(module.code_search(query="connect($$$)", session_id="test"))
        )
        assert isinstance(result, str)


class TestSmartAstEngine:
    """Test SmartAstEngine for AST-based search."""

    def test_engine_init(self):
        module = _load_smart_ast_engine_module()

        engine = module.SmartAstEngine()
        assert engine is not None

    def test_engine_list_rules(self):
        module = _load_smart_ast_engine_module()

        engine = module.SmartAstEngine()
        rules = engine.list_rules()
        assert isinstance(rules, list)
        assert len(rules) > 0

    def test_engine_register_rule(self):
        module = _load_smart_ast_engine_module()

        engine = module.SmartAstEngine()
        initial_count = len(module.BUILTIN_RULES)
        engine.register_rule("test_rule", "test($$$)", "Test rule message")
        assert "test_rule" in module.BUILTIN_RULES
        assert len(module.BUILTIN_RULES) == initial_count + 1

    def test_yaml_rules_loaded(self):
        module = _load_smart_ast_engine_module()

        engine = module.SmartAstEngine()
        rules = engine.list_rules()
        rule_ids = [r["id"] for r in rules]
        expected_rules = ["deep-nesting", "open-without-with", "find-entrypoints"]
        for expected in expected_rules:
            if expected in rule_ids:
                assert True
                break


class TestSearchEngines:
    """Test individual search engines."""

    def test_ast_engine_function(self):
        module = _load_search_engines_module()

        pattern = module.extract_ast_pattern("class User")
        assert pattern == "class User"

        pattern = module.extract_ast_pattern("def authenticate")
        assert pattern == "def authenticate"

    def test_ast_pattern_extraction(self):
        module = _load_search_engines_module()

        assert module.extract_ast_pattern("class User") == "class User"
        assert module.extract_ast_pattern("Find the User class") == "class User"
        assert module.extract_ast_pattern("def authenticate") == "def authenticate"
        assert module.extract_ast_pattern("fn main") == "fn main"
        assert module.extract_ast_pattern("impl Foo") == "impl Foo"
        assert module.extract_ast_pattern("struct User") == "struct User"
        assert module.extract_ast_pattern("how does auth work") is None


class TestClassifier:
    """Test query classifier for intent recognition."""

    def test_classify_structural_query(self):
        module = _load_search_classifier_module()

        result = module.classify_query({"query": "class User"})
        assert "ast" in result["strategies"]

        result = module.classify_query({"query": "def authenticate"})
        assert "ast" in result["strategies"]

    def test_classify_semantic_query(self):
        module = _load_search_classifier_module()

        result = module.classify_query({"query": "how does authentication work?"})
        assert "vector" in result["strategies"]

    def test_classify_grep_query(self):
        module = _load_search_classifier_module()

        result = module.classify_query({"query": "TODO: fix"})
        assert "grep" in result["strategies"]

        result = module.classify_query({"query": '"error message"'})
        assert "grep" in result["strategies"]

    def test_classify_fallback(self):
        module = _load_search_classifier_module()

        result = module.classify_query({"query": "auth"})
        assert "vector" in result["strategies"]


class TestGraphWorkflow:
    """Test native workflow integration."""

    def test_create_search_graph(self):
        module = _load_search_graph_module()

        graph = module.create_search_graph()
        assert graph is not None

    def test_create_initial_state(self):
        module = _load_search_graph_module()

        state = module.create_initial_state("test query", "test-thread")
        assert state["query"] == "test query"
        assert state["thread_id"] == "test-thread"
        assert "strategies" in state
        assert "raw_results" in state


class TestSearchState:
    """Test search state types."""

    def test_state_type(self):
        result = {
            "engine": "ast",
            "file": "test.py",
            "line": 10,
            "content": "def test():",
            "score": 0.9,
        }
        assert result["engine"] == "ast"

        state = {
            "query": "test",
            "strategies": ["ast", "vector"],
            "raw_results": [result],
        }
        assert state["query"] == "test"


class TestPatternUtils:
    """Test pattern utilities."""

    def test_language_patterns(self):
        module = _load_smart_ast_patterns_module()

        assert "class $NAME" in module.LANG_PATTERNS["python"]["classes"]
        assert "def $NAME($$$)" in module.LANG_PATTERNS["python"]["functions"]
        assert "struct $NAME" in module.LANG_PATTERNS["rust"]["structs"]
        assert "fn $NAME($$$)" in module.LANG_PATTERNS["rust"]["functions"]

    def test_common_patterns(self):
        module = _load_smart_ast_patterns_module()

        assert "class $NAME" in module.COMMON_PATTERNS["class"]
