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
fn test_wendao_search_path_fuzzy_emits_section_context() -> Result<(), Box<dyn std::error::Error>> {
    let tmp = TempDir::new()?;
    write_file(
        &tmp.path().join("docs/architecture/graph.md"),
        "# Architecture\n\n## Graph Engine\n\nDetails.\n",
    )?;
    write_file(
        &tmp.path().join("docs/misc.md"),
        "# Misc\n\nGraph mention.\n",
    )?;

    let output = wendao_cmd()
        .arg("--root")
        .arg(tmp.path())
        .arg("search")
        .arg("architecture graph engine")
        .arg("--limit")
        .arg("5")
        .arg("--match-strategy")
        .arg("path_fuzzy")
        .output()?;

    assert!(
        output.status.success(),
        "wendao search with path_fuzzy failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8(output.stdout)?;
    let payload: Value = serde_json::from_str(&stdout)?;
    assert_eq!(
        payload.get("match_strategy").and_then(Value::as_str),
        Some("path_fuzzy")
    );
    let rows = payload
        .get("results")
        .and_then(Value::as_array)
        .ok_or("missing results")?;
    assert!(!rows.is_empty());
    assert_eq!(
        rows.first()
            .and_then(|row| row.get("path"))
            .and_then(Value::as_str),
        Some("docs/architecture/graph.md")
    );
    assert!(
        rows.first()
            .and_then(|row| row.get("best_section"))
            .and_then(Value::as_str)
            .is_some_and(|v| v.contains("Graph Engine"))
    );
    Ok(())
}
