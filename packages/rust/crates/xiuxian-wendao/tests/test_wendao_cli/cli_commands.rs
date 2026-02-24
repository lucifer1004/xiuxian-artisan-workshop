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
fn test_wendao_stats_reports_note_counts() -> Result<(), Box<dyn std::error::Error>> {
    let tmp = TempDir::new()?;
    write_file(&tmp.path().join("docs/a.md"), "# A\n\n[[b]]\n")?;
    write_file(&tmp.path().join("docs/b.md"), "# B\n\n[[a]]\n")?;

    let output = wendao_cmd()
        .arg("--root")
        .arg(tmp.path())
        .arg("stats")
        .output()?;

    assert!(
        output.status.success(),
        "wendao stats failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8(output.stdout)?;
    let payload: Value = serde_json::from_str(&stdout)?;
    assert_eq!(payload.get("total_notes").and_then(Value::as_u64), Some(2));
    assert_eq!(
        payload.get("links_in_graph").and_then(Value::as_u64),
        Some(2)
    );

    Ok(())
}

#[test]
fn test_wendao_allows_global_root_after_subcommand() -> Result<(), Box<dyn std::error::Error>> {
    let tmp = TempDir::new()?;
    write_file(&tmp.path().join("docs/a.md"), "# Alpha\n\n[[b]]\n")?;
    write_file(&tmp.path().join("docs/b.md"), "# Beta\n\n[[a]]\n")?;

    let output = wendao_cmd()
        .arg("search")
        .arg("alpha")
        .arg("--root")
        .arg(tmp.path())
        .arg("--limit")
        .arg("2")
        .output()?;

    assert!(
        output.status.success(),
        "wendao search with trailing --root failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8(output.stdout)?;
    let payload: Value = serde_json::from_str(&stdout)?;
    assert_eq!(payload.get("query").and_then(Value::as_str), Some("alpha"));
    assert!(
        payload
            .get("results")
            .and_then(Value::as_array)
            .is_some_and(|rows| !rows.is_empty())
    );
    Ok(())
}

#[test]
fn test_wendao_hmas_validate_command() -> Result<(), Box<dyn std::error::Error>> {
    let tmp = TempDir::new()?;
    write_file(
        &tmp.path().join("thread.md"),
        r#"
#### [CONCLUSION]
```json
{
  "requirement_id": "REQ-CLI-1",
  "summary": "CLI validator smoke test",
  "confidence_score": 0.9,
  "hard_constraints_checked": ["RULE"]
}
```

#### [DIGITAL THREAD]
```json
{
  "requirement_id": "REQ-CLI-1",
  "source_nodes_accessed": [{"node_id": "note-1"}],
  "hard_constraints_checked": ["RULE"],
  "confidence_score": 0.9
}
```
"#,
    )?;

    let output = wendao_cmd()
        .arg("hmas")
        .arg("validate")
        .arg("--file")
        .arg(tmp.path().join("thread.md"))
        .output()?;

    assert!(
        output.status.success(),
        "wendao hmas validate failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8(output.stdout)?;
    let payload: Value = serde_json::from_str(&stdout)?;
    assert_eq!(payload.get("valid").and_then(Value::as_bool), Some(true));
    assert_eq!(
        payload.get("digital_thread_count").and_then(Value::as_u64),
        Some(1)
    );

    Ok(())
}
