#![allow(
    missing_docs,
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::doc_markdown,
    clippy::implicit_clone,
    clippy::uninlined_format_args,
    clippy::float_cmp,
    clippy::cast_lossless,
    clippy::cast_precision_loss,
    clippy::cast_sign_loss,
    clippy::cast_possible_truncation,
    clippy::manual_string_new,
    clippy::needless_raw_string_hashes,
    clippy::format_push_string,
    clippy::map_unwrap_or,
    clippy::unnecessary_to_owned,
    clippy::too_many_lines
)]
use super::*;

#[test]
fn test_link_graph_search_options_validate_rejects_invalid_tree_filters() {
    let payload = json!({
        "match_strategy": "fts",
        "case_sensitive": false,
        "sort_terms": [{"field": "score", "order": "desc"}],
        "filters": {
            "max_heading_level": 9,
            "per_doc_section_cap": 0
        }
    });
    let parsed: LinkGraphSearchOptions =
        serde_json::from_value(payload).expect("payload should deserialize before validation");
    let error = parsed
        .validate()
        .expect_err("validation should reject invalid tree filters");
    assert!(error.contains("max_heading_level") || error.contains("per_doc_section_cap"));
}
