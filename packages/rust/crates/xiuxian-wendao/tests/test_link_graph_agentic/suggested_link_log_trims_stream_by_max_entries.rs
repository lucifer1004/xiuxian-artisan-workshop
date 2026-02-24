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
fn test_suggested_link_log_trims_stream_by_max_entries() -> Result<(), Box<dyn std::error::Error>> {
    let prefix = unique_prefix();
    if clear_prefix(&prefix).is_err() {
        return Ok(());
    }

    for idx in 0..3 {
        let _ = valkey_suggested_link_log_with_valkey(
            LinkGraphSuggestedLinkRequest {
                source_id: format!("docs/source-{idx}.md"),
                target_id: format!("docs/target-{idx}.md"),
                relation: "related_to".to_string(),
                confidence: 0.5,
                evidence: "test".to_string(),
                agent_id: "qianhuan-architect".to_string(),
                created_at_unix: Some(1_700_000_000.0 + idx as f64),
            },
            TEST_VALKEY_URL,
            Some(&prefix),
            Some(2),
            None,
        )
        .map_err(|err| err.to_string())?;
    }

    let rows = valkey_suggested_link_recent_with_valkey(10, TEST_VALKEY_URL, Some(&prefix))
        .map_err(|err| err.to_string())?;
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0].source_id, "docs/source-2.md");
    assert_eq!(rows[1].source_id, "docs/source-1.md");

    clear_prefix(&prefix)?;
    Ok(())
}
