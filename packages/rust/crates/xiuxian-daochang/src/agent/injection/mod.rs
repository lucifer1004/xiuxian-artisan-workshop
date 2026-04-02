mod assembler;
mod builder;
mod render;

use anyhow::{Context, Result};
use xiuxian_qianhuan::{InjectionPolicy, InjectionSnapshot};

use crate::session::ChatMessage;

pub(super) struct InjectionNormalizationResult {
    pub(super) snapshot: Option<InjectionSnapshot>,
    pub(super) messages: Vec<ChatMessage>,
}

pub(super) fn normalize_messages_with_snapshot(
    session_id: &str,
    turn_id: u64,
    messages: Vec<ChatMessage>,
    policy: InjectionPolicy,
) -> Result<InjectionNormalizationResult> {
    let extraction = builder::extract_blocks(session_id, turn_id, messages);
    if extraction.blocks.is_empty() {
        return Ok(InjectionNormalizationResult {
            snapshot: None,
            messages: extraction.passthrough_messages,
        });
    }

    let snapshot = assembler::assemble_snapshot(session_id, turn_id, policy, extraction.blocks);
    snapshot
        .validate()
        .map_err(anyhow::Error::msg)
        .context("invalid typed injection snapshot")?;

    let mut merged_messages = render::render_snapshot_messages(&snapshot);
    merged_messages.extend(extraction.passthrough_messages);

    Ok(InjectionNormalizationResult {
        snapshot: Some(snapshot),
        messages: merged_messages,
    })
}
