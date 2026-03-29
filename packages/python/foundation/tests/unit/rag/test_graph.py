"""
Tests for xiuxian_rag.graph module.
"""

from unittest.mock import MagicMock, patch

import pytest


class TestBilingualPrompts:
    """Test bilingual entity extraction prompts."""

    def test_english_prompt_exists(self):
        """Test English extraction prompt is defined."""
        from xiuxian_rag.graph import EXTRACT_ENTITIES_PROMPT_EN

        assert EXTRACT_ENTITIES_PROMPT_EN is not None
        assert "You are an expert" in EXTRACT_ENTITIES_PROMPT_EN
        assert "PERSON" in EXTRACT_ENTITIES_PROMPT_EN
        assert "ORGANIZATION" in EXTRACT_ENTITIES_PROMPT_EN

    def test_chinese_prompt_exists(self):
        """Test Chinese extraction prompt is defined."""
        from xiuxian_rag.graph import EXTRACT_ENTITIES_PROMPT_ZH

        assert EXTRACT_ENTITIES_PROMPT_ZH is not None
        assert "实体" in EXTRACT_ENTITIES_PROMPT_ZH
        assert "关系" in EXTRACT_ENTITIES_PROMPT_ZH
        assert "PERSON" in EXTRACT_ENTITIES_PROMPT_ZH

    def test_default_prompt_is_bilingual(self):
        """Test default prompt contains bilingual instructions."""
        from xiuxian_rag.graph import EXTRACT_ENTITIES_PROMPT

        assert EXTRACT_ENTITIES_PROMPT is not None
        # Contains English
        assert "You are an expert" in EXTRACT_ENTITIES_PROMPT
        # Contains Chinese
        assert "中英文双语实体提取" in EXTRACT_ENTITIES_PROMPT
        # Contains both entity type descriptions
        assert "Individual people" in EXTRACT_ENTITIES_PROMPT
        assert "个人、开发人员" in EXTRACT_ENTITIES_PROMPT

    def test_english_prompt_has_json_format(self):
        """Test English prompt has JSON output format."""
        from xiuxian_rag.graph import EXTRACT_ENTITIES_PROMPT_EN

        assert '"entities"' in EXTRACT_ENTITIES_PROMPT_EN
        assert '"relations"' in EXTRACT_ENTITIES_PROMPT_EN
        assert '"name"' in EXTRACT_ENTITIES_PROMPT_EN
        assert '"entity_type"' in EXTRACT_ENTITIES_PROMPT_EN

    def test_chinese_prompt_has_json_format(self):
        """Test Chinese prompt has JSON output format."""
        from xiuxian_rag.graph import EXTRACT_ENTITIES_PROMPT_ZH

        assert "entities" in EXTRACT_ENTITIES_PROMPT_ZH
        assert "relations" in EXTRACT_ENTITIES_PROMPT_ZH
        assert "entity_type" in EXTRACT_ENTITIES_PROMPT_ZH

    def test_all_exports_present(self):
        """Test all prompts are exported."""
        from xiuxian_rag import graph

        assert hasattr(graph, "EXTRACT_ENTITIES_PROMPT")
        assert hasattr(graph, "EXTRACT_ENTITIES_PROMPT_EN")
        assert hasattr(graph, "EXTRACT_ENTITIES_PROMPT_ZH")


class TestKnowledgeGraphExtractor:
    """Test KnowledgeGraphExtractor class."""

    def test_extractor_initialization(self):
        """Test extractor initialization with default config."""
        from xiuxian_rag.graph import KnowledgeGraphExtractor

        with patch("xiuxian_rag.graph.get_rag_config") as mock_get_config:
            mock_config = MagicMock()
            mock_config.knowledge_graph.entity_types = ["PERSON", "ORGANIZATION"]
            mock_config.knowledge_graph.relation_types = ["WORKS_FOR", "PART_OF"]
            mock_config.knowledge_graph.max_entities_per_doc = 50
            mock_config.knowledge_graph.store_in_rust = False
            mock_get_config.return_value = mock_config

            extractor = KnowledgeGraphExtractor()
            assert extractor.entity_types == ["PERSON", "ORGANIZATION"]
            assert extractor.relation_types == ["WORKS_FOR", "PART_OF"]

    def test_extractor_with_custom_types(self):
        """Test extractor with custom entity/relation types."""
        from xiuxian_rag.graph import KnowledgeGraphExtractor

        extractor = KnowledgeGraphExtractor(
            entity_types=["TOOL", "PROJECT"],
            relation_types=["DEPENDS_ON", "USES"],
        )

        assert extractor.entity_types == ["TOOL", "PROJECT"]
        assert extractor.relation_types == ["DEPENDS_ON", "USES"]

    @pytest.mark.asyncio
    async def test_extract_entities_empty_text(self):
        """Test extraction with empty text returns empty lists."""
        from xiuxian_rag.graph import KnowledgeGraphExtractor

        extractor = KnowledgeGraphExtractor()
        entities, relations = await extractor.extract_entities("")

        assert entities == []
        assert relations == []

    @pytest.mark.asyncio
    async def test_extract_entities_no_llm(self):
        """Test extraction without LLM function returns empty."""
        from xiuxian_rag.graph import KnowledgeGraphExtractor

        extractor = KnowledgeGraphExtractor(llm_complete_func=None)
        entities, relations = await extractor.extract_entities("Some text")

        assert entities == []
        assert relations == []

    def test_get_stats(self):
        """Test getting extractor statistics."""
        from xiuxian_rag.graph import KnowledgeGraphExtractor

        extractor = KnowledgeGraphExtractor(
            entity_types=["PERSON", "TOOL"],
            relation_types=["USES"],
        )

        stats = extractor.get_stats()

        assert "entity_types" in stats
        assert "relation_types" in stats
        assert stats["entity_types"] == ["PERSON", "TOOL"]
        assert stats["rust_backend_available"] is False


class TestExtractEntitiesPrompt:
    """Test extraction prompt constants."""

    def test_prompt_exists(self):
        """Test that extraction prompt is defined."""
        from xiuxian_rag.graph import EXTRACT_ENTITIES_PROMPT

        assert EXTRACT_ENTITIES_PROMPT is not None
        assert "entities" in EXTRACT_ENTITIES_PROMPT.lower()
        assert "relations" in EXTRACT_ENTITIES_PROMPT.lower()

    def test_prompt_contains_entity_types(self):
        """Test prompt mentions entity types."""
        from xiuxian_rag.graph import EXTRACT_ENTITIES_PROMPT

        prompt_lower = EXTRACT_ENTITIES_PROMPT.lower()
        assert "PERSON" in prompt_lower or "person" in prompt_lower


class TestGraphExtractorFactory:
    """Test factory functions."""

    def test_get_graph_extractor_disabled(self):
        """Test get_graph_extractor returns None when disabled."""
        with patch("xiuxian_rag.graph.is_knowledge_graph_enabled", return_value=False):
            from xiuxian_rag.graph import get_graph_extractor

            result = get_graph_extractor()
            assert result is None

    def test_get_graph_extractor_enabled(self):
        """Test get_graph_extractor returns extractor when enabled."""
        from xiuxian_rag.graph import KnowledgeGraphExtractor

        with patch("xiuxian_rag.graph.is_knowledge_graph_enabled", return_value=True):
            with patch("xiuxian_rag.graph.get_rag_config") as mock_get_config:
                mock_config = MagicMock()
                mock_config.knowledge_graph.entity_types = ["PERSON"]
                mock_config.knowledge_graph.relation_types = ["WORKS_FOR"]
                mock_config.knowledge_graph.max_entities_per_doc = 100
                mock_config.knowledge_graph.store_in_rust = False
                mock_get_config.return_value = mock_config

                from xiuxian_rag.graph import get_graph_extractor

                result = get_graph_extractor(llm_complete_func=MagicMock())
                assert result is not None
                assert isinstance(result, KnowledgeGraphExtractor)


class TestGraphModuleExports:
    """Test module exports."""

    def test_all_exports_present(self):
        """Test all expected exports are available."""
        from xiuxian_rag import graph

        assert hasattr(graph, "KnowledgeGraphExtractor")
        assert hasattr(graph, "EXTRACT_ENTITIES_PROMPT")
        assert hasattr(graph, "get_graph_extractor")

    def test_all_in_all(self):
        """Test all exports are in __all__."""
        from xiuxian_rag.graph import __all__

        expected = [
            "KnowledgeGraphExtractor",
            "EXTRACT_ENTITIES_PROMPT",
            "EXTRACT_ENTITIES_PROMPT_EN",
            "EXTRACT_ENTITIES_PROMPT_ZH",
            "get_graph_extractor",
        ]

        for item in expected:
            assert item in __all__, f"{item} not in __all__"


class TestBilingualParsing:
    """Test parsing of bilingual (Chinese/English) entity extraction responses."""

    def test_parse_mixed_language_response(self):
        """Test parsing response with mixed Chinese/English entities."""
        from xiuxian_rag.graph import KnowledgeGraphExtractor

        extractor = KnowledgeGraphExtractor()

        # Simulate a response with both English and Chinese entities
        mock_response = """
        {
            "entities": [
                {
                    "name": "Python",
                    "entity_type": "SKILL",
                    "description": "Programming language",
                    "aliases": ["python"]
                },
                {
                    "name": "Claude Code",
                    "entity_type": "TOOL",
                    "description": "AI coding assistant",
                    "aliases": ["Claude"]
                },
                {
                    "name": "Omni Dev Fusion",
                    "entity_type": "PROJECT",
                    "description": "Development environment",
                    "aliases": ["Omni"]
                }
            ],
            "relations": [
                {
                    "source": "Claude Code",
                    "target": "Python",
                    "relation_type": "USES",
                    "description": "Claude Code uses Python"
                }
            ]
        }
        """

        entities, relations = extractor._parse_extraction(mock_response, "test.md")

        assert len(entities) == 3
        entity_names = [e.name for e in entities]
        assert "Python" in entity_names
        assert "Claude Code" in entity_names
        assert "Omni Dev Fusion" in entity_names

        assert len(relations) == 1
        assert relations[0].source == "Claude Code"
        assert relations[0].target == "Python"

    def test_parse_chinese_entities_response(self):
        """Test parsing response with Chinese entities."""
        from xiuxian_rag.graph import KnowledgeGraphExtractor

        extractor = KnowledgeGraphExtractor()

        mock_response = """
        {
            "entities": [
                {
                    "name": "张三",
                    "entity_type": "PERSON",
                    "description": "开发人员"
                },
                {
                    "name": "百度",
                    "entity_type": "ORGANIZATION",
                    "description": "中国互联网公司"
                }
            ],
            "relations": [
                {
                    "source": "张三",
                    "target": "百度",
                    "relation_type": "WORKS_FOR",
                    "description": "在百度工作"
                }
            ]
        }
        """

        entities, relations = extractor._parse_extraction(mock_response, "test.md")

        assert len(entities) == 2
        entity_names = [e.name for e in entities]
        assert "张三" in entity_names
        assert "百度" in entity_names

        assert len(relations) == 1
        assert relations[0].source == "张三"
        assert relations[0].target == "百度"

    def test_parse_empty_response(self):
        """Test parsing empty response."""
        from xiuxian_rag.graph import KnowledgeGraphExtractor

        extractor = KnowledgeGraphExtractor()

        entities, relations = extractor._parse_extraction("{}", "test.md")

        assert entities == []
        assert relations == []

    def test_parse_invalid_json(self):
        """Test parsing invalid JSON response."""
        from xiuxian_rag.graph import KnowledgeGraphExtractor

        extractor = KnowledgeGraphExtractor()

        entities, relations = extractor._parse_extraction("not valid json", "test.md")

        assert entities == []
        assert relations == []


class TestPromptFormat:
    """Test prompt format and content."""

    def test_prompt_format_variable(self):
        """Test that prompt contains format variable placeholder."""
        from xiuxian_rag.graph import (
            EXTRACT_ENTITIES_PROMPT,
            EXTRACT_ENTITIES_PROMPT_EN,
            EXTRACT_ENTITIES_PROMPT_ZH,
        )

        # All prompts should contain the {text} placeholder
        assert "{text}" in EXTRACT_ENTITIES_PROMPT
        assert "{text}" in EXTRACT_ENTITIES_PROMPT_EN
        assert "{text}" in EXTRACT_ENTITIES_PROMPT_ZH

    def test_prompt_entity_type_coverage(self):
        """Test that prompts cover all standard entity types."""
        from xiuxian_rag.graph import EXTRACT_ENTITIES_PROMPT

        # Check for standard entity types
        entity_types = ["PERSON", "ORGANIZATION", "CONCEPT", "PROJECT", "TOOL", "SKILL"]
        for et in entity_types:
            assert et in EXTRACT_ENTITIES_PROMPT, f"Missing {et} in prompt"

    def test_prompt_relation_type_coverage(self):
        """Test that prompts cover key relation types."""
        from xiuxian_rag.graph import EXTRACT_ENTITIES_PROMPT

        # Check for key relation types
        relation_types = ["WORKS_FOR", "PART_OF", "USES", "DEPENDS_ON", "CREATED_BY"]
        for rt in relation_types:
            assert rt in EXTRACT_ENTITIES_PROMPT, f"Missing {rt} in prompt"
