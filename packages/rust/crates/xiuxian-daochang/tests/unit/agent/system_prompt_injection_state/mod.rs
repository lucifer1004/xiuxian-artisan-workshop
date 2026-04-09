use anyhow::Result;

use super::{
    SessionSystemPromptInjectionSnapshot, message_to_snapshot,
    normalize_session_prompt_injection_snapshot, snapshot_to_message,
};

#[test]
fn normalization_counts_qa_entries_and_renders_canonical_xml() -> Result<()> {
    let snapshot = normalize_session_prompt_injection_snapshot(
        r"
<system_prompt_injection>
  <qa><q>q1</q><a>a1</a></qa>
  <qa><q>q2</q><a>a2</a></qa>
</system_prompt_injection>
",
    )?;

    assert_eq!(snapshot.qa_count, 2);
    assert!(snapshot.xml.contains("<system_prompt_injection>"));
    assert!(snapshot.xml.contains("<q>q1</q>"));
    assert!(snapshot.updated_at_unix_ms > 0);
    Ok(())
}

#[test]
fn snapshot_storage_roundtrip_preserves_fields() -> Result<()> {
    let snapshot = SessionSystemPromptInjectionSnapshot {
        xml: "<system_prompt_injection><qa><q>q</q><a>a</a></qa></system_prompt_injection>"
            .to_string(),
        qa_count: 1,
        updated_at_unix_ms: 42,
    };
    let message = snapshot_to_message(&snapshot)
        .ok_or_else(|| anyhow::anyhow!("snapshot serialization should succeed"))?;
    let decoded = message_to_snapshot(&message)
        .ok_or_else(|| anyhow::anyhow!("snapshot payload should decode"))?;
    assert_eq!(decoded, snapshot);
    Ok(())
}
