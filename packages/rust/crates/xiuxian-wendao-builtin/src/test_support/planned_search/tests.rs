use xiuxian_wendao_julia::integration_support::{
    julia_planned_search_openai_runtime_config_toml,
    julia_planned_search_similarity_only_runtime_config_toml,
    julia_planned_search_vector_store_runtime_config_toml,
};

use super::{
    LinkedBuiltinWendaoArrowScoreRow,
    linked_builtin_julia_planned_search_openai_runtime_config_toml,
    linked_builtin_julia_planned_search_similarity_only_runtime_config_toml,
    linked_builtin_julia_planned_search_vector_store_runtime_config_toml,
};

#[test]
fn linked_builtin_planned_search_runtime_config_helpers_match_julia_source_of_truth() {
    assert_eq!(
        linked_builtin_julia_planned_search_openai_runtime_config_toml(
            "/tmp/vector-store",
            "http://127.0.0.1:9999",
            "http://127.0.0.1:8088",
        ),
        julia_planned_search_openai_runtime_config_toml(
            "/tmp/vector-store",
            "http://127.0.0.1:9999",
            "http://127.0.0.1:8088",
        )
    );
    assert_eq!(
        linked_builtin_julia_planned_search_vector_store_runtime_config_toml(
            "/tmp/vector-store",
            "http://127.0.0.1:8088",
        ),
        julia_planned_search_vector_store_runtime_config_toml(
            "/tmp/vector-store",
            "http://127.0.0.1:8088",
        )
    );
    assert_eq!(
        linked_builtin_julia_planned_search_similarity_only_runtime_config_toml(
            "/tmp/vector-store",
            "http://127.0.0.1:8088",
        ),
        julia_planned_search_similarity_only_runtime_config_toml(
            "/tmp/vector-store",
            "http://127.0.0.1:8088",
        )
    );
}

#[test]
fn linked_builtin_custom_score_row_alias_preserves_fixture_shape() {
    let row = LinkedBuiltinWendaoArrowScoreRow {
        doc_id: "alpha",
        analyzer_score: 0.4,
        final_score: 0.9,
    };

    assert_eq!(row.doc_id, "alpha");
    assert!((row.analyzer_score - 0.4).abs() < f64::EPSILON);
    assert!((row.final_score - 0.9).abs() < f64::EPSILON);
}
