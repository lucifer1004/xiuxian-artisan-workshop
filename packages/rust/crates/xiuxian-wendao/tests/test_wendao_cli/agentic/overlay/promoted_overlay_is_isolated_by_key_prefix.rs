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
fn test_wendao_promoted_overlay_is_isolated_by_key_prefix() -> Result<(), Box<dyn std::error::Error>>
{
    let tmp = TempDir::new()?;
    write_file(&tmp.path().join("docs/a.md"), "# A\n\nalpha\n")?;
    write_file(&tmp.path().join("docs/b.md"), "# B\n\nbeta\n")?;

    let prefix_a = unique_agentic_prefix();
    let prefix_b = unique_agentic_prefix();
    if clear_valkey_prefix(&prefix_a).is_err() || clear_valkey_prefix(&prefix_b).is_err() {
        return Ok(());
    }

    let config_a = tmp.path().join("wendao.a.yaml");
    let config_b = tmp.path().join("wendao.b.yaml");
    fs::write(
        &config_a,
        format!(
            "link_graph:\n  cache:\n    valkey_url: \"redis://127.0.0.1:6379/0\"\n    key_prefix: \"{prefix_a}\"\n  agentic:\n    suggested_link:\n      max_entries: 64\n      ttl_seconds: null\n"
        ),
    )?;
    fs::write(
        &config_b,
        format!(
            "link_graph:\n  cache:\n    valkey_url: \"redis://127.0.0.1:6379/0\"\n    key_prefix: \"{prefix_b}\"\n  agentic:\n    suggested_link:\n      max_entries: 64\n      ttl_seconds: null\n"
        ),
    )?;

    let log_output = wendao_cmd()
        .arg("--conf")
        .arg(&config_a)
        .arg("agentic")
        .arg("log")
        .arg("docs/a.md")
        .arg("docs/b.md")
        .arg("related_to")
        .arg("--confidence")
        .arg("0.9")
        .arg("--evidence")
        .arg("prefix-a-only")
        .arg("--agent-id")
        .arg("qianhuan-architect")
        .output()?;
    assert!(
        log_output.status.success(),
        "wendao agentic log failed: {}",
        String::from_utf8_lossy(&log_output.stderr)
    );
    let suggestion_id = serde_json::from_str::<Value>(&String::from_utf8(log_output.stdout)?)?
        .get("suggestion_id")
        .and_then(Value::as_str)
        .ok_or("missing suggestion_id")?
        .to_string();

    let decide_output = wendao_cmd()
        .arg("--conf")
        .arg(&config_a)
        .arg("agentic")
        .arg("decide")
        .arg(&suggestion_id)
        .arg("--target-state")
        .arg("promoted")
        .arg("--decided-by")
        .arg("omega-gate")
        .arg("--reason")
        .arg("prefix isolation test")
        .output()?;
    assert!(
        decide_output.status.success(),
        "wendao agentic decide failed: {}",
        String::from_utf8_lossy(&decide_output.stderr)
    );

    let search_a = wendao_cmd()
        .arg("--root")
        .arg(tmp.path())
        .arg("--conf")
        .arg(&config_a)
        .arg("search")
        .arg("alpha")
        .arg("--limit")
        .arg("5")
        .output()?;
    assert!(
        search_a.status.success(),
        "wendao search for prefix_a failed: {}",
        String::from_utf8_lossy(&search_a.stderr)
    );
    let payload_a: Value = serde_json::from_str(&String::from_utf8(search_a.stdout)?)?;
    assert_eq!(
        payload_a
            .get("promoted_overlay")
            .and_then(|row| row.get("applied"))
            .and_then(Value::as_bool),
        Some(true)
    );

    let search_b = wendao_cmd()
        .arg("--root")
        .arg(tmp.path())
        .arg("--conf")
        .arg(&config_b)
        .arg("search")
        .arg("alpha")
        .arg("--limit")
        .arg("5")
        .output()?;
    assert!(
        search_b.status.success(),
        "wendao search for prefix_b failed: {}",
        String::from_utf8_lossy(&search_b.stderr)
    );
    let payload_b: Value = serde_json::from_str(&String::from_utf8(search_b.stdout)?)?;
    assert_eq!(
        payload_b
            .get("promoted_overlay")
            .and_then(|row| row.get("applied"))
            .and_then(Value::as_bool),
        Some(false)
    );

    clear_valkey_prefix(&prefix_a)?;
    clear_valkey_prefix(&prefix_b)?;
    Ok(())
}
