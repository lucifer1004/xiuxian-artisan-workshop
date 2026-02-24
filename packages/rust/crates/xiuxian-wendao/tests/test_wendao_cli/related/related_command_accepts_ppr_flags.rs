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
fn test_wendao_related_command_accepts_ppr_flags() -> Result<(), Box<dyn std::error::Error>> {
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
        "wendao related with ppr flags failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let payload: Value = serde_json::from_str(&String::from_utf8(output.stdout)?)?;
    let rows = payload
        .as_array()
        .ok_or("expected related output to be a json array")?;
    assert_eq!(rows.len(), 3);
    let stems: Vec<&str> = rows
        .iter()
        .filter_map(|row| row.get("stem").and_then(Value::as_str))
        .collect();
    assert!(stems.contains(&"a"));
    assert!(stems.contains(&"c"));
    assert!(stems.contains(&"d"));

    Ok(())
}
