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
fn test_suggested_link_log_rejects_invalid_payload() {
    let prefix = unique_prefix();
    let result = valkey_suggested_link_log_with_valkey(
        LinkGraphSuggestedLinkRequest {
            source_id: "".to_string(),
            target_id: "docs/b.md".to_string(),
            relation: "related_to".to_string(),
            confidence: 0.4,
            evidence: "test".to_string(),
            agent_id: "qianhuan-architect".to_string(),
            created_at_unix: None,
        },
        TEST_VALKEY_URL,
        Some(&prefix),
        Some(10),
        None,
    );
    assert!(result.is_err());
}
