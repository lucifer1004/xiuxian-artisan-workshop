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
fn test_wendao_search_rejects_legacy_sort_flag() -> Result<(), Box<dyn std::error::Error>> {
    let tmp = TempDir::new()?;
    write_file(&tmp.path().join("docs/a.md"), "# A\n")?;

    let output = wendao_cmd()
        .arg("--root")
        .arg(tmp.path())
        .arg("search")
        .arg("a")
        .arg("--sort")
        .arg("path_asc")
        .output()?;

    assert!(
        !output.status.success(),
        "legacy --sort flag should be rejected, but command succeeded"
    );
    let stderr = String::from_utf8(output.stderr)?;
    assert!(stderr.contains("unexpected argument '--sort'"));
    assert!(stderr.contains("--sort-term"));
    Ok(())
}
