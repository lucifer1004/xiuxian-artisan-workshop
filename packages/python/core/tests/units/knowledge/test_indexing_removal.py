from __future__ import annotations

import pytest

from xiuxian_core.knowledge.analyzer import analyze_knowledge, get_knowledge_dataframe
from xiuxian_core.knowledge.dependency_indexer import DependencyIndexer
from xiuxian_core.knowledge.ingestion import FileIngestor
from xiuxian_core.knowledge.symbol_indexer import SymbolIndexer, build_symbol_index


def test_file_ingestor_is_removed():
    with pytest.raises(RuntimeError, match="Python knowledge ingestion has been removed"):
        FileIngestor()


def test_knowledge_analyzer_is_removed():
    with pytest.raises(RuntimeError, match="Python knowledge analytics have been removed"):
        get_knowledge_dataframe()

    with pytest.raises(RuntimeError, match="Python knowledge analytics have been removed"):
        analyze_knowledge()


def test_symbol_indexer_is_removed():
    with pytest.raises(RuntimeError, match="Python symbol indexing has been removed"):
        SymbolIndexer()

    with pytest.raises(RuntimeError, match="Python symbol indexing has been removed"):
        build_symbol_index(".")


def test_dependency_indexer_is_removed():
    with pytest.raises(RuntimeError, match="Python dependency indexing has been removed"):
        DependencyIndexer()
