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
fn test_suggested_link_log_roundtrip() -> Result<(), Box<dyn std::error::Error>> {
    let prefix = unique_prefix();
    if clear_prefix(&prefix).is_err() {
        return Ok(());
    }

    let entry = valkey_suggested_link_log_with_valkey(
        LinkGraphSuggestedLinkRequest {
            source_id: "docs/a.md".to_string(),
            target_id: "docs/b.md".to_string(),
            relation: "implements".to_string(),
            confidence: 0.83,
            evidence: "bridge signal from architecture section".to_string(),
            agent_id: "qianhuan-architect".to_string(),
            created_at_unix: Some(1_700_000_000.0),
        },
        TEST_VALKEY_URL,
        Some(&prefix),
        Some(10),
        Some(60),
    )
    .map_err(|err| err.to_string())?;
    assert_eq!(
        entry.promotion_state,
        LinkGraphSuggestedLinkState::Provisional
    );
    assert!(!entry.suggestion_id.trim().is_empty());
    assert_eq!(entry.source_id, "docs/a.md");
    assert_eq!(entry.updated_at_unix, entry.created_at_unix);

    let rows = valkey_suggested_link_recent_with_valkey(10, TEST_VALKEY_URL, Some(&prefix))
        .map_err(|err| err.to_string())?;
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0], entry);

    clear_prefix(&prefix)?;
    Ok(())
}
