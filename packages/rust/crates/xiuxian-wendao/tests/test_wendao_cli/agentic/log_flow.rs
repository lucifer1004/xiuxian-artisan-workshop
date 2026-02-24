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
fn test_wendao_agentic_log_recent_decide_flow() -> Result<(), Box<dyn std::error::Error>> {
    let tmp = TempDir::new()?;
    let prefix = unique_agentic_prefix();
    if clear_valkey_prefix(&prefix).is_err() {
        return Ok(());
    }

    let config_path = tmp.path().join("wendao.yaml");
    fs::write(
        &config_path,
        format!(
            "link_graph:\n  cache:\n    valkey_url: \"redis://127.0.0.1:6379/0\"\n    key_prefix: \"{prefix}\"\n  agentic:\n    suggested_link:\n      max_entries: 64\n      ttl_seconds: null\n"
        ),
    )?;

    let log_output = wendao_cmd()
        .arg("--conf")
        .arg(&config_path)
        .arg("agentic")
        .arg("log")
        .arg("docs/a.md")
        .arg("docs/b.md")
        .arg("implements")
        .arg("--confidence")
        .arg("0.8")
        .arg("--evidence")
        .arg("bridge found")
        .arg("--agent-id")
        .arg("qianhuan-architect")
        .arg("--created-at-unix")
        .arg("1700000300")
        .output()?;
    assert!(
        log_output.status.success(),
        "wendao agentic log failed: {}",
        String::from_utf8_lossy(&log_output.stderr)
    );
    let log_stdout = String::from_utf8(log_output.stdout)?;
    let logged: Value = serde_json::from_str(&log_stdout)?;
    let suggestion_id = logged
        .get("suggestion_id")
        .and_then(Value::as_str)
        .ok_or("missing suggestion_id")?
        .to_string();
    assert_eq!(
        logged.get("promotion_state").and_then(Value::as_str),
        Some("provisional")
    );

    let recent_output = wendao_cmd()
        .arg("--conf")
        .arg(&config_path)
        .arg("agentic")
        .arg("recent")
        .arg("--limit")
        .arg("10")
        .arg("--latest")
        .arg("--state")
        .arg("provisional")
        .output()?;
    assert!(
        recent_output.status.success(),
        "wendao agentic recent failed: {}",
        String::from_utf8_lossy(&recent_output.stderr)
    );
    let recent_stdout = String::from_utf8(recent_output.stdout)?;
    let recent_rows: Value = serde_json::from_str(&recent_stdout)?;
    let recent_rows = recent_rows
        .as_array()
        .ok_or("recent payload must be array")?;
    assert_eq!(recent_rows.len(), 1);
    assert_eq!(
        recent_rows[0].get("suggestion_id").and_then(Value::as_str),
        Some(suggestion_id.as_str())
    );

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
        .arg("validated")
        .arg("--decided-at-unix")
        .arg("1700000310")
        .output()?;
    assert!(
        decide_output.status.success(),
        "wendao agentic decide failed: {}",
        String::from_utf8_lossy(&decide_output.stderr)
    );
    let decide_stdout = String::from_utf8(decide_output.stdout)?;
    let decide_payload: Value = serde_json::from_str(&decide_stdout)?;
    assert_eq!(
        decide_payload
            .get("suggestion")
            .and_then(|row| row.get("promotion_state"))
            .and_then(Value::as_str),
        Some("promoted")
    );

    let decisions_output = wendao_cmd()
        .arg("--conf")
        .arg(&config_path)
        .arg("agentic")
        .arg("decisions")
        .arg("--limit")
        .arg("10")
        .output()?;
    assert!(
        decisions_output.status.success(),
        "wendao agentic decisions failed: {}",
        String::from_utf8_lossy(&decisions_output.stderr)
    );
    let decisions_stdout = String::from_utf8(decisions_output.stdout)?;
    let decisions_rows: Value = serde_json::from_str(&decisions_stdout)?;
    let decisions_rows = decisions_rows
        .as_array()
        .ok_or("decisions payload must be array")?;
    assert_eq!(decisions_rows.len(), 1);
    assert_eq!(
        decisions_rows[0]
            .get("suggestion_id")
            .and_then(Value::as_str),
        Some(suggestion_id.as_str())
    );
    assert_eq!(
        decisions_rows[0]
            .get("target_state")
            .and_then(Value::as_str),
        Some("promoted")
    );

    clear_valkey_prefix(&prefix)?;
    Ok(())
}
