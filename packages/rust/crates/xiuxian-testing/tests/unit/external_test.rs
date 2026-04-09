use super::*;
use std::fs;

fn create_temp_crate() -> tempfile::TempDir {
    let temp = match tempfile::tempdir() {
        Ok(temp) => temp,
        Err(error) => panic!("tempdir should be created: {error}"),
    };
    if let Err(error) = fs::create_dir_all(temp.path().join("src")) {
        panic!("src dir should be created: {error}");
    }
    if let Err(error) = fs::write(
        temp.path().join("Cargo.toml"),
        "[package]\nname = \"fixture\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    ) {
        panic!("Cargo.toml should be written: {error}");
    }
    temp
}

fn write_fixture_file(crate_root: &Path, relative_path: &str, content: &str) {
    let path = crate_root.join(relative_path);
    let Some(parent) = path.parent() else {
        panic!("fixture path should have parent: {path:?}");
    };
    if let Err(error) = fs::create_dir_all(parent) {
        panic!("fixture directories should be created: {error}");
    }
    if let Err(error) = fs::write(path, content) {
        panic!("fixture file should be written: {error}");
    }
}

#[test]
fn test_calculate_test_path_simple() {
    let source = Path::new("src/foo.rs");
    let test_path = calculate_test_path(source, "tests");
    assert_eq!(test_path, PathBuf::from("tests/unit/foo.rs"));
}

#[test]
fn test_calculate_test_path_nested() {
    let source = Path::new("src/foo/bar/baz.rs");
    let test_path = calculate_test_path(source, "tests");
    assert_eq!(test_path, PathBuf::from("tests/unit/foo/bar/baz.rs"));
}

#[test]
fn test_generate_path_attribute_simple() {
    let source = Path::new("src/foo.rs");
    let path_attr = generate_path_attribute(source, "tests");
    assert_eq!(path_attr, "../tests/unit/foo.rs");
}

#[test]
fn test_generate_path_attribute_nested() {
    let source = Path::new("src/foo/bar/baz.rs");
    let path_attr = generate_path_attribute(source, "tests");
    assert_eq!(path_attr, "../../../tests/unit/foo/bar/baz.rs");
}

#[test]
fn validate_external_test_mounts_reports_inline_test_blocks() {
    let temp = create_temp_crate();
    write_fixture_file(
        temp.path(),
        "src/foo.rs",
        r"
#[cfg(test)]
mod tests {
    #[test]
    fn keeps_policy_strict() {}
}
",
    );

    let issues = validate_external_test_mounts(temp.path());
    assert_eq!(issues.len(), 1);

    match &issues[0] {
        ExternalTestValidationIssue::InlineTestBlock {
            source_file,
            module_name,
            suggested_path,
            ..
        } => {
            assert!(source_file.ends_with("src/foo.rs"));
            assert_eq!(module_name, "tests");
            assert_eq!(suggested_path, "../tests/unit/foo.rs");
        }
        issue => panic!("expected inline test issue, got {issue:?}"),
    }
}

#[test]
fn validate_external_test_mounts_reports_named_test_modules() {
    let temp = create_temp_crate();
    write_fixture_file(
        temp.path(),
        "src/parser.rs",
        r#"
#[cfg(any(test, feature = "nightly-tests"))]
pub(crate) mod parser_tests {
    #[test]
    fn parses() {}
}
"#,
    );

    let issues = validate_external_test_mounts(temp.path());
    assert_eq!(issues.len(), 1);
    assert!(matches!(
        issues[0],
        ExternalTestValidationIssue::InlineTestBlock { .. }
    ));
}

#[test]
fn validate_external_test_mounts_reports_cfg_test_modules_without_path_mounts() {
    let temp = create_temp_crate();
    write_fixture_file(
        temp.path(),
        "src/parser.rs",
        r"
#[cfg(test)]
mod parser_tests;
",
    );

    let issues = validate_external_test_mounts(temp.path());
    assert_eq!(issues.len(), 1);

    match &issues[0] {
        ExternalTestValidationIssue::SourceResidentTestModule {
            source_file,
            module_name,
            suggested_path,
            ..
        } => {
            assert!(source_file.ends_with("src/parser.rs"));
            assert_eq!(module_name, "parser_tests");
            assert_eq!(suggested_path, "../tests/unit/parser.rs");
        }
        issue => panic!("expected source-resident test module issue, got {issue:?}"),
    }
}

#[test]
fn validate_external_test_mounts_ignores_test_api_and_resolves_nested_paths() {
    let temp = create_temp_crate();
    write_fixture_file(
        temp.path(),
        "src/foo/bar.rs",
        r#"
fn helper() {}

#[cfg(test)]
pub mod test_api {
    pub use super::helper;
}

#[cfg(test)]
#[path = "../../tests/unit/foo/bar.rs"]
mod tests;
"#,
    );
    write_fixture_file(
        temp.path(),
        "tests/unit/foo/bar.rs",
        "#[test]\nfn uses_external_test_file() {}\n",
    );

    let issues = validate_external_test_mounts(temp.path());
    assert!(issues.is_empty(), "expected no issues, got {issues:?}");
}

#[test]
fn validate_external_test_mounts_reports_missing_external_test_file() {
    let temp = create_temp_crate();
    write_fixture_file(
        temp.path(),
        "src/foo/bar.rs",
        r#"
#[cfg(test)]
#[path = "../../tests/unit/foo/bar.rs"]
mod tests;
"#,
    );

    let issues = validate_external_test_mounts(temp.path());
    assert_eq!(issues.len(), 1);
    assert!(matches!(
        issues[0],
        ExternalTestValidationIssue::MissingTestFile { .. }
    ));
}

#[test]
fn validate_external_test_mounts_ignores_doc_comment_examples() {
    let temp = create_temp_crate();
    write_fixture_file(
        temp.path(),
        "src/lib.rs",
        r#"
//! ```ignore
//! #[cfg(test)]
//! #[path = "../../tests/unit/foo/bar.rs"]
//! mod tests;
//! ```
"#,
    );

    let issues = validate_external_test_mounts(temp.path());
    assert!(issues.is_empty(), "expected no issues, got {issues:?}");
}

#[test]
fn detect_inline_test_blocks_respects_min_block_lines() {
    let temp = create_temp_crate();
    write_fixture_file(
        temp.path(),
        "src/foo.rs",
        r"
#[cfg(test)]
mod tests {
    #[test]
    fn tiny() {}
}
",
    );

    assert!(detect_inline_test_blocks(temp.path(), 10).is_empty());
    assert_eq!(detect_inline_test_blocks(temp.path(), 1).len(), 1);
}
