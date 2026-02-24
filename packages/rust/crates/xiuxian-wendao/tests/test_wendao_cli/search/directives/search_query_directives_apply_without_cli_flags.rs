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
fn test_wendao_search_query_directives_apply_without_cli_flags()
-> Result<(), Box<dyn std::error::Error>> {
    let tmp = TempDir::new()?;
    write_file(&tmp.path().join("docs/a.md"), "# A\n\n[[b]]\n")?;
    write_file(&tmp.path().join("docs/c.md"), "# C\n\n[[b]]\n")?;
    write_file(&tmp.path().join("docs/b.md"), "# B\n\nNo links.\n")?;

    let output = wendao_cmd()
        .arg("--root")
        .arg(tmp.path())
        .arg("search")
        .arg("to:b sort:path_asc .md")
        .arg("--limit")
        .arg("10")
        .output()?;

    assert!(
        output.status.success(),
        "wendao search with query directives failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let payload: Value = serde_json::from_str(&String::from_utf8(output.stdout)?)?;
    assert_eq!(payload.get("query").and_then(Value::as_str), Some(".md"));
    let filters = payload.get("filters").ok_or("missing filters payload")?;
    assert_eq!(
        filters
            .get("link_to")
            .and_then(|row| row.get("seeds"))
            .and_then(Value::as_array)
            .map(std::vec::Vec::len),
        Some(1)
    );
    let sort_terms = payload
        .get("sort_terms")
        .and_then(Value::as_array)
        .ok_or("missing sort_terms")?;
    assert_eq!(sort_terms.len(), 1);
    assert_eq!(
        sort_terms[0].get("field").and_then(Value::as_str),
        Some("path")
    );
    assert_eq!(
        sort_terms[0].get("order").and_then(Value::as_str),
        Some("asc")
    );
    let rows = payload
        .get("results")
        .and_then(Value::as_array)
        .ok_or("missing results")?;
    assert_eq!(rows.len(), 2);
    assert_eq!(
        rows.first()
            .and_then(|row| row.get("path"))
            .and_then(Value::as_str),
        Some("docs/a.md")
    );
    assert_eq!(
        rows.get(1)
            .and_then(|row| row.get("path"))
            .and_then(Value::as_str),
        Some("docs/c.md")
    );
    Ok(())
}
