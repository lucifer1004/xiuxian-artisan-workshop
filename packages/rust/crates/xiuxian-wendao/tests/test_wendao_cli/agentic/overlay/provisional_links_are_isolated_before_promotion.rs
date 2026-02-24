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
fn test_wendao_provisional_links_are_isolated_before_promotion()
-> Result<(), Box<dyn std::error::Error>> {
    let tmp = TempDir::new()?;
    write_file(&tmp.path().join("docs/a.md"), "# A\n\nalpha\n")?;
    write_file(&tmp.path().join("docs/b.md"), "# B\n\nbeta\n")?;

    let prefix = unique_agentic_prefix();
    if clear_valkey_prefix(&prefix).is_err() {
        return Ok(());
    }

    let config_path = tmp.path().join("wendao.yaml");
    fs::write(
        &config_path,
        format!(
            "link_graph:\n  cache:\n    valkey_url: \"redis://127.0.0.1:6379/0\"\n    key_prefix: \"{prefix}\"\n  agentic:\n    suggested_link:\n      max_entries: 64\n      ttl_seconds: null\n    search:\n      include_provisional_default: false\n      provisional_limit: 10\n"
        ),
    )?;

    let log_output = wendao_cmd()
        .arg("--conf")
        .arg(&config_path)
        .arg("agentic")
        .arg("log")
        .arg("docs/a.md")
        .arg("docs/b.md")
        .arg("related_to")
        .arg("--confidence")
        .arg("0.92")
        .arg("--evidence")
        .arg("provisional-only")
        .arg("--agent-id")
        .arg("qianhuan-architect")
        .output()?;
    assert!(
        log_output.status.success(),
        "wendao agentic log failed: {}",
        String::from_utf8_lossy(&log_output.stderr)
    );

    let neighbors_output = wendao_cmd()
        .arg("--root")
        .arg(tmp.path())
        .arg("--conf")
        .arg(&config_path)
        .arg("neighbors")
        .arg("a")
        .arg("--direction")
        .arg("outgoing")
        .arg("--hops")
        .arg("1")
        .arg("--limit")
        .arg("10")
        .arg("--verbose")
        .output()?;
    assert!(
        neighbors_output.status.success(),
        "wendao neighbors --verbose failed: {}",
        String::from_utf8_lossy(&neighbors_output.stderr)
    );
    let neighbors_payload: Value =
        serde_json::from_str(&String::from_utf8(neighbors_output.stdout)?)?;
    let neighbors_rows = neighbors_payload
        .get("results")
        .and_then(Value::as_array)
        .ok_or("neighbors verbose payload missing results")?;
    assert!(
        !neighbors_rows
            .iter()
            .any(|row| row.get("stem").and_then(Value::as_str) == Some("b")),
        "provisional link leaked into neighbors traversal before promotion: payload={neighbors_payload}"
    );
    let neighbors_overlay = neighbors_payload
        .get("promoted_overlay")
        .ok_or("neighbors payload missing promoted_overlay")?;
    assert_eq!(
        neighbors_overlay.get("applied").and_then(Value::as_bool),
        Some(false)
    );

    let related_output = wendao_cmd()
        .arg("--root")
        .arg(tmp.path())
        .arg("--conf")
        .arg(&config_path)
        .arg("related")
        .arg("a")
        .arg("--max-distance")
        .arg("2")
        .arg("--limit")
        .arg("10")
        .arg("--verbose")
        .output()?;
    assert!(
        related_output.status.success(),
        "wendao related --verbose failed: {}",
        String::from_utf8_lossy(&related_output.stderr)
    );
    let related_payload: Value = serde_json::from_str(&String::from_utf8(related_output.stdout)?)?;
    let related_rows = related_payload
        .get("results")
        .and_then(Value::as_array)
        .ok_or("related verbose payload missing results")?;
    assert!(
        !related_rows
            .iter()
            .any(|row| row.get("stem").and_then(Value::as_str) == Some("b")),
        "provisional link leaked into related traversal before promotion: payload={related_payload}"
    );
    let related_overlay = related_payload
        .get("promoted_overlay")
        .ok_or("related payload missing promoted_overlay")?;
    assert_eq!(
        related_overlay.get("applied").and_then(Value::as_bool),
        Some(false)
    );

    let search_output = wendao_cmd()
        .arg("--root")
        .arg(tmp.path())
        .arg("--conf")
        .arg(&config_path)
        .arg("search")
        .arg("alpha")
        .arg("--limit")
        .arg("10")
        .output()?;
    assert!(
        search_output.status.success(),
        "wendao search failed: {}",
        String::from_utf8_lossy(&search_output.stderr)
    );
    let search_payload: Value = serde_json::from_str(&String::from_utf8(search_output.stdout)?)?;
    let search_rows = search_payload
        .get("results")
        .and_then(Value::as_array)
        .ok_or("search payload missing results")?;
    assert!(
        !search_rows
            .iter()
            .any(|row| row.get("stem").and_then(Value::as_str) == Some("b")),
        "provisional link leaked into default search results before promotion: payload={search_payload}"
    );
    assert_eq!(
        search_payload
            .get("promoted_overlay")
            .and_then(|row| row.get("applied"))
            .and_then(Value::as_bool),
        Some(false)
    );

    clear_valkey_prefix(&prefix)?;
    Ok(())
}
