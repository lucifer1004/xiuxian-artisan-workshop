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
fn test_saliency_store_auto_repairs_invalid_payload() -> Result<(), Box<dyn std::error::Error>> {
    let prefix = unique_prefix();
    if clear_prefix(&prefix).is_err() {
        return Ok(());
    }

    let _ = valkey_saliency_touch_with_valkey(
        LinkGraphSaliencyTouchRequest {
            node_id: "note-b".to_string(),
            activation_delta: 1,
            saliency_base: Some(5.0),
            decay_rate: Some(0.01),
            alpha: None,
            minimum_saliency: None,
            maximum_saliency: None,
            now_unix: Some(1_700_000_000),
        },
        TEST_VALKEY_URL,
        Some(&prefix),
    )
    .map_err(|err| err.to_string())?;

    let mut conn = valkey_connection()?;
    let pattern = format!("{prefix}:saliency:*");
    let keys: Vec<String> = redis::cmd("KEYS").arg(&pattern).query(&mut conn)?;
    if keys.is_empty() {
        clear_prefix(&prefix)?;
        return Ok(());
    }
    let key = keys[0].clone();
    redis::cmd("SET")
        .arg(&key)
        .arg("{\"schema\":\"invalid.schema\"}")
        .query::<()>(&mut conn)?;

    let fetched = valkey_saliency_get_with_valkey("note-b", TEST_VALKEY_URL, Some(&prefix))
        .map_err(|err| err.to_string())?;
    assert!(fetched.is_none());

    let raw: Option<String> = redis::cmd("GET").arg(&key).query(&mut conn)?;
    assert!(raw.is_none(), "invalid payload key should be removed");

    clear_prefix(&prefix)?;
    Ok(())
}
