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

mod promoted_links_materialize_in_neighbors_and_related;
mod promoted_overlay_is_isolated_by_key_prefix;
mod promoted_overlay_resolves_mixed_alias_forms;
mod provisional_links_are_isolated_before_promotion;

fn write_agentic_config(
    config_path: &Path,
    prefix: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    fs::write(
        config_path,
        format!(
            "link_graph:\n  cache:\n    valkey_url: \"redis://127.0.0.1:6379/0\"\n    key_prefix: \"{prefix}\"\n  agentic:\n    suggested_link:\n      max_entries: 64\n      ttl_seconds: null\n"
        ),
    )?;
    Ok(())
}

fn assert_promoted_overlay_applied(payload: &Value) -> Result<(), Box<dyn std::error::Error>> {
    let overlay = payload
        .get("promoted_overlay")
        .ok_or("missing promoted_overlay telemetry")?;
    assert_eq!(overlay.get("applied").and_then(Value::as_bool), Some(true));
    assert_eq!(
        overlay.get("source").and_then(Value::as_str),
        Some("valkey.suggested_link_recent_latest")
    );
    Ok(())
}

fn assert_verbose_overlay(payload: &Value) -> Result<(), Box<dyn std::error::Error>> {
    assert_promoted_overlay_applied(payload)?;
    assert!(
        payload
            .get("phases")
            .and_then(Value::as_array)
            .is_some_and(|rows| rows.iter().any(|row| {
                row.get("phase").and_then(Value::as_str) == Some("link_graph.overlay.promoted")
            }))
    );
    assert!(
        payload
            .get("monitor")
            .and_then(|row| row.get("bottlenecks"))
            .and_then(|row| row.get("slowest_phase"))
            .is_some()
    );
    Ok(())
}
