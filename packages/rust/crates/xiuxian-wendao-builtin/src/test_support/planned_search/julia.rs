use xiuxian_wendao_julia::integration_support::{
    JuliaExampleServiceGuard, WendaoArrowScoreRow, julia_planned_search_openai_runtime_config_toml,
    julia_planned_search_similarity_only_runtime_config_toml,
    julia_planned_search_vector_store_runtime_config_toml,
    spawn_wendaoanalyzer_similarity_only_service, spawn_wendaoanalyzer_stream_linear_blend_service,
    spawn_wendaoarrow_custom_scoring_service, spawn_wendaoarrow_stream_metadata_service,
    spawn_wendaoarrow_stream_scoring_service,
};

/// Linked builtin alias for the custom `WendaoArrow` score-row fixture.
pub type LinkedBuiltinWendaoArrowScoreRow<'a> = WendaoArrowScoreRow<'a>;

/// Render the linked builtin OpenAI-compatible planned-search runtime-config
/// fixture.
#[must_use]
pub fn linked_builtin_julia_planned_search_openai_runtime_config_toml(
    vector_store_path: &str,
    embedding_base_url: &str,
    rerank_base_url: &str,
) -> String {
    julia_planned_search_openai_runtime_config_toml(
        vector_store_path,
        embedding_base_url,
        rerank_base_url,
    )
}

/// Render the linked builtin vector-store planned-search runtime-config
/// fixture.
#[must_use]
pub fn linked_builtin_julia_planned_search_vector_store_runtime_config_toml(
    vector_store_path: &str,
    rerank_base_url: &str,
) -> String {
    julia_planned_search_vector_store_runtime_config_toml(vector_store_path, rerank_base_url)
}

/// Render the linked builtin analyzer-backed similarity-only planned-search
/// runtime-config fixture.
#[must_use]
pub fn linked_builtin_julia_planned_search_similarity_only_runtime_config_toml(
    vector_store_path: &str,
    rerank_base_url: &str,
) -> String {
    julia_planned_search_similarity_only_runtime_config_toml(vector_store_path, rerank_base_url)
}

/// Spawn the linked builtin custom `WendaoArrow` scoring service fixture.
pub async fn linked_builtin_spawn_wendaoarrow_custom_scoring_service(
    rows: &[LinkedBuiltinWendaoArrowScoreRow<'_>],
) -> (String, JuliaExampleServiceGuard) {
    spawn_wendaoarrow_custom_scoring_service(rows).await
}

/// Spawn the linked builtin official `WendaoArrow` stream-metadata example.
pub async fn linked_builtin_spawn_wendaoarrow_stream_metadata_service()
-> (String, JuliaExampleServiceGuard) {
    spawn_wendaoarrow_stream_metadata_service().await
}

/// Spawn the linked builtin official `WendaoArrow` stream-scoring example.
pub async fn linked_builtin_spawn_wendaoarrow_stream_scoring_service()
-> (String, JuliaExampleServiceGuard) {
    spawn_wendaoarrow_stream_scoring_service().await
}

/// Spawn the linked builtin `WendaoAnalyzer` linear-blend example service.
pub async fn linked_builtin_spawn_wendaoanalyzer_stream_linear_blend_service()
-> (String, JuliaExampleServiceGuard) {
    spawn_wendaoanalyzer_stream_linear_blend_service().await
}

/// Spawn the linked builtin analyzer-backed similarity-only example service.
pub async fn linked_builtin_spawn_wendaoanalyzer_similarity_only_service()
-> (String, JuliaExampleServiceGuard) {
    spawn_wendaoanalyzer_similarity_only_service().await
}
