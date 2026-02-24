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
fn test_wendao_promoted_links_materialize_in_neighbors_and_related()
-> Result<(), Box<dyn std::error::Error>> {
    let tmp = TempDir::new()?;
    write_file(&tmp.path().join("docs/a.md"), "# A\n\nalpha\n")?;
    write_file(&tmp.path().join("docs/b.md"), "# B\n\nbeta\n")?;

    let prefix = unique_agentic_prefix();
    if clear_valkey_prefix(&prefix).is_err() {
        return Ok(());
    }

    let config_path = tmp.path().join("wendao.yaml");
    write_agentic_config(&config_path, &prefix)?;

    let log_output = wendao_cmd()
        .arg("--conf")
        .arg(&config_path)
        .arg("agentic")
        .arg("log")
        .arg("docs/a.md")
        .arg("docs/b.md")
        .arg("related_to")
        .arg("--confidence")
        .arg("0.91")
        .arg("--evidence")
        .arg("promoted-link-test")
        .arg("--agent-id")
        .arg("qianhuan-architect")
        .output()?;
    assert!(
        log_output.status.success(),
        "wendao agentic log failed: {}",
        String::from_utf8_lossy(&log_output.stderr)
    );
    let log_stdout = String::from_utf8(log_output.stdout)?;
    let log_payload: Value = serde_json::from_str(&log_stdout)?;
    let suggestion_id = log_payload
        .get("suggestion_id")
        .and_then(Value::as_str)
        .ok_or("missing suggestion_id")?
        .to_string();

    let decide_output = wendao_cmd()
        .arg("--conf")
        .arg(&config_path)
        .arg("agentic")
        .arg("decide")
        .arg(&suggestion_id)
        .arg("--target-state")
        .arg("promoted")
        .arg("--decided-by")
        .arg("omega-gate")
        .arg("--reason")
        .arg("promotion for retrieval overlay")
        .output()?;
    assert!(
        decide_output.status.success(),
        "wendao agentic decide failed: {}",
        String::from_utf8_lossy(&decide_output.stderr)
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
        .output()?;
    assert!(
        neighbors_output.status.success(),
        "wendao neighbors failed: {}",
        String::from_utf8_lossy(&neighbors_output.stderr)
    );
    let neighbors_stdout = String::from_utf8(neighbors_output.stdout)?;
    let neighbors_payload: Value = serde_json::from_str(&neighbors_stdout)?;
    let neighbors = neighbors_payload
        .as_array()
        .ok_or("neighbors payload must be array")?;
    assert!(neighbors.iter().any(|row| {
        row.get("stem").and_then(Value::as_str) == Some("b")
            && row.get("path").and_then(Value::as_str) == Some("docs/b.md")
    }));

    let neighbors_verbose_output = wendao_cmd()
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
        neighbors_verbose_output.status.success(),
        "wendao neighbors --verbose failed: {}",
        String::from_utf8_lossy(&neighbors_verbose_output.stderr)
    );
    let neighbors_verbose_stdout = String::from_utf8(neighbors_verbose_output.stdout)?;
    let neighbors_verbose_payload: Value = serde_json::from_str(&neighbors_verbose_stdout)?;
    assert_verbose_overlay(&neighbors_verbose_payload)?;

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
        .output()?;
    assert!(
        related_output.status.success(),
        "wendao related failed: {}",
        String::from_utf8_lossy(&related_output.stderr)
    );
    let related_stdout = String::from_utf8(related_output.stdout)?;
    let related_payload: Value = serde_json::from_str(&related_stdout)?;
    let related_rows = related_payload
        .as_array()
        .ok_or("related payload must be array")?;
    assert!(
        related_rows
            .iter()
            .any(|row| row.get("stem").and_then(Value::as_str) == Some("b")),
        "expected promoted edge to affect related traversal: payload={related_payload}"
    );

    let search_output = wendao_cmd()
        .arg("--root")
        .arg(tmp.path())
        .arg("--conf")
        .arg(&config_path)
        .arg("search")
        .arg("alpha")
        .arg("--limit")
        .arg("5")
        .output()?;
    assert!(
        search_output.status.success(),
        "wendao search failed: {}",
        String::from_utf8_lossy(&search_output.stderr)
    );
    let search_stdout = String::from_utf8(search_output.stdout)?;
    let search_payload: Value = serde_json::from_str(&search_stdout)?;
    assert_promoted_overlay_applied(&search_payload)?;
    let overlay = search_payload
        .get("promoted_overlay")
        .ok_or("missing promoted_overlay telemetry")?;
    assert!(
        overlay
            .get("promoted_rows")
            .and_then(Value::as_u64)
            .unwrap_or(0)
            >= 1
    );
    assert!(
        overlay
            .get("added_edges")
            .and_then(Value::as_u64)
            .unwrap_or(0)
            >= 1
    );

    clear_valkey_prefix(&prefix)?;
    Ok(())
}
