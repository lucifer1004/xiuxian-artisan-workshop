#![allow(
    missing_docs,
    unused_imports,
    dead_code,
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::doc_markdown,
    clippy::uninlined_format_args,
    clippy::float_cmp,
    clippy::field_reassign_with_default,
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_possible_wrap,
    clippy::map_unwrap_or,
    clippy::option_as_ref_deref,
    clippy::unreadable_literal,
    clippy::useless_conversion,
    clippy::match_wildcard_for_single_variants,
    clippy::redundant_closure_for_method_calls,
    clippy::needless_raw_string_hashes,
    clippy::manual_async_fn,
    clippy::manual_let_else,
    clippy::too_many_lines,
    clippy::too_many_arguments,
    clippy::unnecessary_literal_bound,
    clippy::needless_pass_by_value,
    clippy::struct_field_names,
    clippy::single_match_else,
    clippy::similar_names,
    clippy::format_collect,
    clippy::assigning_clones
)]

use axum::http::StatusCode;

use crate::gateway::http::runtime::resolve_embed_model;

#[test]
fn resolve_embed_model_prefers_configured_default_over_request_override() {
    let resolved = resolve_embed_model(
        Some("openai/qwen3-embedding:0.6b"),
        Some("ollama/qwen3-embedding:0.6b"),
    )
    .expect("expected configured default model");
    assert_eq!(resolved, "ollama/qwen3-embedding:0.6b");
}

#[test]
fn resolve_embed_model_uses_requested_when_default_missing() {
    let resolved = resolve_embed_model(Some("openai/text-embedding-3-small"), None)
        .expect("expected request model when no configured default exists");
    assert_eq!(resolved, "openai/text-embedding-3-small");
}

#[test]
fn resolve_embed_model_rejects_when_both_request_and_default_are_missing() {
    let error = resolve_embed_model(None, None).expect_err("expected missing model error");
    assert_eq!(error.0, StatusCode::BAD_REQUEST);
    assert!(error.1.contains("embedding model must be provided"));
}