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
fn test_wendao_related_verbose_includes_diagnostics() -> Result<(), Box<dyn std::error::Error>> {
    let tmp = TempDir::new()?;
    write_file(&tmp.path().join("docs/a.md"), "# A\n\n[[b]]\n")?;
    write_file(&tmp.path().join("docs/b.md"), "# B\n\n[[c]]\n")?;
    write_file(&tmp.path().join("docs/c.md"), "# C\n\n[[d]]\n")?;
    write_file(&tmp.path().join("docs/d.md"), "# D\n\nNo links.\n")?;

    let output = wendao_cmd()
        .arg("--root")
        .arg(tmp.path())
        .arg("related")
        .arg("b")
        .arg("--max-distance")
        .arg("2")
        .arg("--limit")
        .arg("10")
        .arg("--verbose")
        .arg("--ppr-alpha")
        .arg("0.9")
        .arg("--ppr-max-iter")
        .arg("64")
        .arg("--ppr-tol")
        .arg("1e-6")
        .arg("--ppr-subgraph-mode")
        .arg("force")
        .output()?;

    assert!(
        output.status.success(),
        "wendao related --verbose failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let payload: Value = serde_json::from_str(&String::from_utf8(output.stdout)?)?;
    assert_eq!(payload.get("stem").and_then(Value::as_str), Some("b"));
    assert_eq!(payload.get("max_distance").and_then(Value::as_u64), Some(2));
    assert_eq!(payload.get("limit").and_then(Value::as_u64), Some(10));
    let ppr = payload.get("ppr").ok_or("missing ppr payload")?;
    assert_eq!(ppr.get("alpha").and_then(Value::as_f64), Some(0.9));
    assert_eq!(ppr.get("max_iter").and_then(Value::as_u64), Some(64));
    assert_eq!(ppr.get("tol").and_then(Value::as_f64), Some(1e-6));
    assert_eq!(
        ppr.get("subgraph_mode").and_then(Value::as_str),
        Some("force")
    );
    assert_related_verbose_diagnostics(&payload)?;
    assert_related_verbose_monitor(&payload)?;

    let rows = payload
        .get("results")
        .and_then(Value::as_array)
        .ok_or("expected verbose results array")?;
    assert_eq!(rows.len(), 3);
    assert_eq!(payload.get("total").and_then(Value::as_u64), Some(3));

    Ok(())
}
