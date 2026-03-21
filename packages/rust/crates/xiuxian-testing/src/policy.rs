//! Unified crate-level test policy validation.
//!
//! This module combines structure validation and external-test policy validation
//! into a single reusable entry point for consumer crates.

use std::fmt::Write;
use std::fs;
use std::path::Path;

use serde::Deserialize;

use crate::external_test::{ExternalTestValidationIssue, validate_external_test_mounts};
use crate::validation::{
    StructureViolation, TestsStructurePolicy, format_violation_report,
    validate_crate_tests_with_policy,
};

const TEST_POLICY_CONFIG_FILE: &str = "tests/xiuxian-testings-rules.toml";

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
struct CrateTestPolicyToml {
    tests: TestsPolicyToml,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
struct TestsPolicyToml {
    allowed_root_files: Vec<String>,
    allowed_directories: Vec<String>,
}

/// Full test policy report for a crate.
#[derive(Debug, Default)]
pub struct CrateTestPolicyReport {
    /// External test policy issues found under `src/`.
    pub external_test_issues: Vec<ExternalTestValidationIssue>,
    /// Test directory structure violations found under `tests/`.
    pub structure_violations: Vec<StructureViolation>,
}

impl CrateTestPolicyReport {
    /// Returns true when the crate satisfies both test policy layers.
    #[must_use]
    pub fn is_clean(&self) -> bool {
        self.external_test_issues.is_empty() && self.structure_violations.is_empty()
    }
}

/// Validate the full crate test policy.
#[must_use]
pub fn validate_crate_test_policy(crate_root: &Path) -> CrateTestPolicyReport {
    validate_crate_test_policy_with_structure_policy(crate_root, None)
}

fn validate_crate_test_policy_with_structure_policy(
    crate_root: &Path,
    structure_policy: Option<&TestsStructurePolicy>,
) -> CrateTestPolicyReport {
    CrateTestPolicyReport {
        external_test_issues: validate_external_test_mounts(crate_root),
        structure_violations: validate_crate_tests_with_policy(crate_root, structure_policy),
    }
}

fn normalize_entries(entries: Vec<String>) -> Vec<String> {
    entries
        .into_iter()
        .map(|entry| entry.trim().to_string())
        .filter(|entry| !entry.is_empty())
        .collect()
}

fn load_structure_policy_from_toml(
    crate_root: &Path,
) -> Result<Option<TestsStructurePolicy>, String> {
    let config_path = crate_root.join(TEST_POLICY_CONFIG_FILE);
    if !config_path.exists() {
        return Ok(None);
    }

    let content = fs::read_to_string(&config_path)
        .map_err(|error| format!("Failed to read {}: {error}", config_path.display()))?;
    let parsed: CrateTestPolicyToml = toml::from_str(&content)
        .map_err(|error| format!("Failed to parse {}: {error}", config_path.display()))?;

    Ok(Some(TestsStructurePolicy {
        allowed_root_files: normalize_entries(parsed.tests.allowed_root_files),
        allowed_directories: normalize_entries(parsed.tests.allowed_directories),
    }))
}

/// Validate crate test policy with project-level `tests/xiuxian-testings-rules.toml`
/// overrides.
///
/// This reads `{crate_root}/tests/xiuxian-testings-rules.toml` when present.
pub fn validate_crate_test_policy_with_workspace_config(
    crate_root: &Path,
) -> Result<CrateTestPolicyReport, String> {
    let structure_policy = load_structure_policy_from_toml(crate_root)?;
    Ok(validate_crate_test_policy_with_structure_policy(
        crate_root,
        structure_policy.as_ref(),
    ))
}

/// Validate only the test directory structure with
/// `tests/xiuxian-testings-rules.toml` overrides.
pub fn validate_crate_tests_structure_with_workspace_config(
    crate_root: &Path,
) -> Result<Vec<StructureViolation>, String> {
    let structure_policy = load_structure_policy_from_toml(crate_root)?;
    Ok(validate_crate_tests_with_policy(
        crate_root,
        structure_policy.as_ref(),
    ))
}

/// Format a human-readable full crate test policy report.
#[must_use]
pub fn format_crate_test_policy_report(report: &CrateTestPolicyReport) -> String {
    if report.is_clean() {
        return "✅ Crate test policy is valid.".to_string();
    }

    let mut output = String::new();
    let _ = writeln!(output, "❌ Crate test policy violations detected.\n");

    if !report.external_test_issues.is_empty() {
        let _ = writeln!(output, "External Test Policy:");
        for issue in &report.external_test_issues {
            let _ = writeln!(output, "- {}", issue.description());
        }
        output.push('\n');
    }

    if !report.structure_violations.is_empty() {
        let _ = writeln!(output, "Test Structure Policy:");
        output.push_str(&format_violation_report(&report.structure_violations));
    }

    output
}

/// Assert that a crate satisfies the shared xiuxian test policy.
///
/// # Panics
///
/// Panics when the crate violates the shared xiuxian test policy. The panic message contains the
/// formatted policy report so callers can see every detected violation.
#[track_caller]
pub fn assert_crate_test_policy(crate_root: &Path) {
    let report = validate_crate_test_policy(crate_root);
    assert!(
        report.is_clean(),
        "{}",
        format_crate_test_policy_report(&report)
    );
}

/// Assert crate test policy using optional project-level
/// `tests/xiuxian-testings-rules.toml` overrides.
///
/// # Panics
///
/// Panics when the TOML config cannot be loaded or when the crate violates the configured policy.
#[track_caller]
pub fn assert_crate_test_policy_with_workspace_config(crate_root: &Path) {
    let report = validate_crate_test_policy_with_workspace_config(crate_root)
        .unwrap_or_else(|error| panic!("{error}"));
    assert!(
        report.is_clean(),
        "{}",
        format_crate_test_policy_report(&report)
    );
}

/// Assert only test directory structure using `tests/xiuxian-testings-rules.toml`
/// overrides.
///
/// # Panics
///
/// Panics when TOML config cannot be loaded or when test structure violations are found.
#[track_caller]
pub fn assert_crate_tests_structure_with_workspace_config(crate_root: &Path) {
    let violations = validate_crate_tests_structure_with_workspace_config(crate_root)
        .unwrap_or_else(|error| panic!("{error}"));
    assert!(
        violations.is_empty(),
        "{}",
        format_violation_report(&violations)
    );
}

#[cfg(test)]
mod tests {
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
    fn validate_crate_test_policy_returns_clean_report_for_valid_crate() {
        let temp = create_temp_crate();
        write_fixture_file(
            temp.path(),
            "src/foo.rs",
            r#"
fn helper() {}

#[cfg(test)]
#[path = "../tests/unit/foo.rs"]
mod tests;
"#,
        );
        write_fixture_file(
            temp.path(),
            "tests/unit/foo.rs",
            r"
use super::*;

#[test]
fn helper_exists() {
    helper();
}
",
        );

        let report = validate_crate_test_policy(temp.path());
        assert!(report.is_clean(), "expected clean report, got {report:?}");
    }

    #[test]
    fn validate_crate_test_policy_collects_both_policy_layers() {
        let temp = create_temp_crate();
        write_fixture_file(
            temp.path(),
            "src/foo.rs",
            r"
#[cfg(test)]
mod tests {
    #[test]
    fn inline_policy_violation() {}
}
",
        );
        write_fixture_file(
            temp.path(),
            "tests/test_foo.rs",
            "#[test]\nfn scattered() {}\n",
        );

        let report = validate_crate_test_policy(temp.path());
        assert_eq!(report.external_test_issues.len(), 1);
        assert_eq!(report.structure_violations.len(), 1);

        let formatted = format_crate_test_policy_report(&report);
        assert!(formatted.contains("External Test Policy"));
        assert!(formatted.contains("Test Structure Policy"));
    }

    #[test]
    fn validate_crate_test_policy_with_workspace_config_applies_overrides() {
        let temp = create_temp_crate();
        write_fixture_file(
            temp.path(),
            "tests/coactivation_multihop_diffusion.rs",
            "#[test]\nfn smoke() {}\n",
        );
        write_fixture_file(
            temp.path(),
            "tests/bench/throughput.rs",
            "#[test]\nfn smoke() {}\n",
        );
        write_fixture_file(
            temp.path(),
            TEST_POLICY_CONFIG_FILE,
            r#"
[tests]
allowed_root_files = ["coactivation_multihop_diffusion.rs"]
allowed_directories = ["bench"]
"#,
        );

        let report = validate_crate_test_policy_with_workspace_config(temp.path())
            .unwrap_or_else(|error| panic!("workspace-config validation should pass: {error}"));
        assert!(report.is_clean(), "expected clean report, got {report:?}");
    }

    #[test]
    fn validate_crate_test_policy_with_workspace_config_rejects_invalid_toml() {
        let temp = create_temp_crate();
        write_fixture_file(
            temp.path(),
            TEST_POLICY_CONFIG_FILE,
            r#"
[tests
allowed_root_files = ["coactivation_multihop_diffusion.rs"]
"#,
        );

        let error = validate_crate_test_policy_with_workspace_config(temp.path())
            .expect_err("invalid toml should fail");
        assert!(error.contains(TEST_POLICY_CONFIG_FILE));
    }

    #[test]
    fn assert_crate_tests_structure_with_workspace_config_ignores_external_layer() {
        let temp = create_temp_crate();
        write_fixture_file(
            temp.path(),
            "src/foo.rs",
            r"
#[cfg(test)]
mod tests {
    #[test]
    fn inline_policy_violation() {}
}
",
        );
        write_fixture_file(
            temp.path(),
            "tests/coactivation_weighted_propagation.rs",
            "#[test]\nfn smoke() {}\n",
        );
        write_fixture_file(
            temp.path(),
            TEST_POLICY_CONFIG_FILE,
            r#"
[tests]
allowed_root_files = ["coactivation_weighted_propagation.rs"]
"#,
        );

        assert_crate_tests_structure_with_workspace_config(temp.path());
    }
}
