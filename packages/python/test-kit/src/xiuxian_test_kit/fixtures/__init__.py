"""Pytest fixtures exported by xiuxian_test_kit."""

from xiuxian_test_kit.fixtures.core import (
    cache_dir,
    clean_settings,
    config_dir,
    mock_agent_context,
    project_root,
    test_tracer,
)
from xiuxian_test_kit.fixtures.files import temp_yaml_file
from xiuxian_test_kit.fixtures.git import (
    git_repo,
    git_test_env,
    gitops_verifier,
    temp_git_repo,
)
from xiuxian_test_kit.fixtures.arrow import (
    TABLE_HEALTH_IPC_COLUMNS,
    assert_table_health_ipc_table,
    decode_table_health_ipc_bytes,
    make_table_health_ipc_bytes,
    table_health_ipc_schema,
)
from xiuxian_test_kit.fixtures.rag import (
    RagTestHelper,
    mock_llm_empty_response,
    mock_llm_for_extraction,
    mock_llm_invalid_json,
    rag_config_fixture,
    rag_graph_extractor,
    rag_knowledge_graph_disabled,
    rag_knowledge_graph_enabled,
    rag_paragraph_chunker,
    rag_semantic_chunker,
    rag_sentence_chunker,
    rag_sliding_window_chunker,
    rag_test_helper,
    sample_text_for_chunking,
    sample_text_for_entity_extraction,
)
from xiuxian_test_kit.fixtures.vector import (
    ROUTE_TEST_SCHEMA_V1,
    hybrid_payload_factory,
    make_db_search_hybrid_result_list,
    make_db_search_vector_result_list,
    make_hybrid_payload,
    make_route_test_payload,
    make_router_result_payload,
    make_tool_search_payload,
    make_vector_payload,
    parametrize_input_schema_variants,
    parametrize_route_intent_queries,
    tool_search_payload_factory,
    vector_payload_factory,
    with_removed_key,
)

__all__ = [  # noqa: RUF022
    # Core
    "test_tracer",
    "project_root",
    "config_dir",
    "cache_dir",
    "clean_settings",
    "mock_agent_context",
    # Git
    "temp_git_repo",
    "git_repo",
    "git_test_env",
    "gitops_verifier",
    # Arrow / LanceDB analytics (table health IPC)
    "TABLE_HEALTH_IPC_COLUMNS",
    "assert_table_health_ipc_table",
    "decode_table_health_ipc_bytes",
    "make_table_health_ipc_bytes",
    "table_health_ipc_schema",
    # RAG
    "rag_config_fixture",
    "rag_knowledge_graph_disabled",
    "rag_knowledge_graph_enabled",
    "mock_llm_for_extraction",
    "mock_llm_empty_response",
    "mock_llm_invalid_json",
    "rag_graph_extractor",
    "rag_sentence_chunker",
    "rag_paragraph_chunker",
    "rag_sliding_window_chunker",
    "rag_semantic_chunker",
    "sample_text_for_chunking",
    "sample_text_for_entity_extraction",
    "rag_test_helper",
    "RagTestHelper",
    # Files
    "temp_yaml_file",
    # Vector payload fixtures
    "ROUTE_TEST_SCHEMA_V1",
    "make_db_search_hybrid_result_list",
    "make_db_search_vector_result_list",
    "make_hybrid_payload",
    "make_route_test_payload",
    "make_router_result_payload",
    "make_tool_search_payload",
    "make_vector_payload",
    "tool_search_payload_factory",
    "vector_payload_factory",
    "hybrid_payload_factory",
    "parametrize_input_schema_variants",
    "parametrize_route_intent_queries",
    "with_removed_key",
]
