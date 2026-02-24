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
fn test_wendao_search_can_include_provisional_suggestions() -> Result<(), Box<dyn std::error::Error>>
{
    let tmp = TempDir::new()?;
    write_file(&tmp.path().join("docs/a.md"), "# A\n\nalpha query term\n")?;
    write_file(&tmp.path().join("docs/b.md"), "# B\n\nbeta\n")?;

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
        .arg("related_to")
        .arg("--confidence")
        .arg("0.9")
        .arg("--evidence")
        .arg("alpha bridge")
        .arg("--agent-id")
        .arg("qianhuan-architect")
        .output()?;
    assert!(
        log_output.status.success(),
        "wendao agentic log failed: {}",
        String::from_utf8_lossy(&log_output.stderr)
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
        .arg("--include-provisional")
        .arg("--provisional-limit")
        .arg("10")
        .output()?;
    assert!(
        search_output.status.success(),
        "wendao search with provisional failed: {}",
        String::from_utf8_lossy(&search_output.stderr)
    );

    let stdout = String::from_utf8(search_output.stdout)?;
    let payload: Value = serde_json::from_str(&stdout)?;
    assert_eq!(
        payload.get("provisional_error").and_then(Value::as_str),
        None
    );
    let suggestions = payload
        .get("provisional_suggestions")
        .and_then(Value::as_array)
        .ok_or("missing provisional_suggestions")?;
    assert!(!suggestions.is_empty());
    let results = payload
        .get("results")
        .and_then(Value::as_array)
        .ok_or("missing results")?;
    let injected = results.iter().find(|row| {
        row.get("stem").and_then(Value::as_str) == Some("b")
            && row
                .get("match_reason")
                .and_then(Value::as_str)
                .is_some_and(|reason| reason.contains("agentic_provisional"))
    });
    assert!(
        injected.is_some(),
        "expected provisional hybrid policy to inject/boost docs/b.md: payload={payload}"
    );

    clear_valkey_prefix(&prefix)?;
    Ok(())
}

#[test]
fn test_wendao_search_uses_engine_default_for_provisional_injection()
-> Result<(), Box<dyn std::error::Error>> {
    let tmp = TempDir::new()?;
    write_file(&tmp.path().join("docs/a.md"), "# A\n\nalpha query term\n")?;
    write_file(&tmp.path().join("docs/b.md"), "# B\n\nbeta\n")?;

    let prefix = unique_agentic_prefix();
    if clear_valkey_prefix(&prefix).is_err() {
        return Ok(());
    }
    let config_path = tmp.path().join("wendao.yaml");
    fs::write(
        &config_path,
        format!(
            "link_graph:\n  cache:\n    valkey_url: \"redis://127.0.0.1:6379/0\"\n    key_prefix: \"{prefix}\"\n  agentic:\n    suggested_link:\n      max_entries: 64\n      ttl_seconds: null\n    search:\n      include_provisional_default: true\n      provisional_limit: 10\n"
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
        .arg("--evidence")
        .arg("bridge")
        .arg("--agent-id")
        .arg("qianhuan-architect")
        .output()?;
    assert!(
        log_output.status.success(),
        "wendao agentic log failed: {}",
        String::from_utf8_lossy(&log_output.stderr)
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
        "wendao search with engine default provisional failed: {}",
        String::from_utf8_lossy(&search_output.stderr)
    );

    let stdout = String::from_utf8(search_output.stdout)?;
    let payload: Value = serde_json::from_str(&stdout)?;
    let suggestions = payload
        .get("provisional_suggestions")
        .and_then(Value::as_array)
        .ok_or("missing provisional_suggestions")?;
    assert!(!suggestions.is_empty());

    clear_valkey_prefix(&prefix)?;
    Ok(())
}
