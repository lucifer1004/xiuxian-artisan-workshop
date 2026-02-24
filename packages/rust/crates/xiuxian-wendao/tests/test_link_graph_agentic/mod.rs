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
//! Integration tests for passive LinkGraph suggested-link logging.

use redis::Connection;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};
use xiuxian_wendao::{
    LinkGraphSuggestedLinkDecisionRequest, LinkGraphSuggestedLinkRequest,
    LinkGraphSuggestedLinkState, valkey_suggested_link_decide_with_valkey,
    valkey_suggested_link_decisions_recent_with_valkey, valkey_suggested_link_log_with_valkey,
    valkey_suggested_link_recent_latest_with_valkey, valkey_suggested_link_recent_with_valkey,
};

const TEST_VALKEY_URL: &str = "redis://127.0.0.1:6379/0";
static PREFIX_COUNTER: AtomicU64 = AtomicU64::new(0);

fn unique_prefix() -> String {
    let seq = PREFIX_COUNTER.fetch_add(1, Ordering::Relaxed);
    let pid = std::process::id();
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|value| value.as_nanos())
        .unwrap_or(0);
    format!("xiuxian_wendao:test:suggested_link:{pid}:{nanos}:{seq}")
}

fn valkey_connection() -> Result<Connection, Box<dyn std::error::Error>> {
    let client = redis::Client::open(TEST_VALKEY_URL)?;
    Ok(client.get_connection()?)
}

fn clear_prefix(prefix: &str) -> Result<(), Box<dyn std::error::Error>> {
    let mut conn = valkey_connection()?;
    let pattern = format!("{prefix}:*");
    let keys: Vec<String> = redis::cmd("KEYS").arg(&pattern).query(&mut conn)?;
    if !keys.is_empty() {
        redis::cmd("DEL").arg(keys).query::<()>(&mut conn)?;
    }
    Ok(())
}

mod suggested_link_decide_promoted_with_audit;
mod suggested_link_decide_rejects_invalid_transition;
mod suggested_link_log_rejects_invalid_payload;
mod suggested_link_log_roundtrip;
mod suggested_link_log_trims_stream_by_max_entries;
