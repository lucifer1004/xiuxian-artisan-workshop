from __future__ import annotations

import xiuxian_rag as rag


def test_rag_root_no_longer_exports_helper_facades() -> None:
    for name in (
        "Chunk",
        "Entity",
        "EntityMention",
        "EntityType",
        "ExtractedChunk",
        "Relation",
        "RelationType",
        "RetrievalConfig",
        "RetrievalResult",
        "HybridRetrievalBackend",
        "create_hybrid_node",
        "create_retrieval_backend",
        "create_retriever_node",
        "create_rag_adapter",
        "get_rag_config",
        "get_parser",
        "extract_pdf_images",
    ):
        assert not hasattr(rag, name)
