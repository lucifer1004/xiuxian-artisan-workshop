//! Test directory structure validation.
//!
//! This module provides utilities to validate that test files follow the
//! xiuxian testing conventions, ensuring consistent organization across crates.
//!
//! # Directory Structure Convention
//!
//! ```text
//! tests/
//! ├── scenarios/           # Scenario-based tests (managed by ScenarioFramework)
//! ├── snapshots/           # Insta snapshots (auto-generated)
//! ├── fixtures/            # Test fixtures and data files
//! ├── support/             # Test helper modules
//! ├── unit/                # Unit tests (*.rs, snake_case naming)
//! │   ├── entity.rs
//! │   └── storage.rs
//! ├── integration/         # Integration tests (*.rs, snake_case naming)
//! │   ├── dependency_indexer.rs
//! │   └── link_graph.rs
//! ├── performance/         # Optional performance gates and stress suites
//! ├── scenarios_test.rs    # Scenario test entry point
//! └── xiuxian-testing-gate.rs # Unified test-policy and integration mount gate
//! ```
//!
//! # Naming Conventions
//!
//! - **Unit tests**: `tests/unit/{module}.rs` (e.g., `entity.rs`, `storage.rs`)
//! - **Integration tests**: `tests/integration/{feature}.rs`
//! - **Test entry points**: Explicit root gate files only
//!   (for example `scenarios_test.rs`, `xiuxian-testing-gate.rs`)
//!
//! # Forbidden Patterns
//!
//! - `tests/test_*.rs` → Move to `tests/unit/` or `tests/integration/`
//! - `tests/*_unit.rs` → Move to `tests/unit/{name}.rs`
//! - `tests/*_integration.rs` → Move to `tests/integration/{name}.rs`
//! - Scattered files in `tests/` root → Organize into subdirectories

use std::fs;
use std::path::{Path, PathBuf};

/// Optional structure policy overrides loaded from crate-level configuration.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TestsStructurePolicy {
    /// Additional allowed directories directly under `tests/`.
    pub allowed_directories: Vec<String>,
    /// Additional allowed Rust file names directly under `tests/`.
    pub allowed_root_files: Vec<String>,
}

/// Represents a violation of the test directory structure convention.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StructureViolation {
    /// The file or directory that violates the convention.
    pub path: PathBuf,
    /// The type of violation.
    pub kind: ViolationKind,
    /// Suggested fix for the violation.
    pub suggestion: String,
}

/// The type of structure violation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ViolationKind {
    /// File uses `test_` prefix in tests root (should be in unit/ or integration/).
    TestPrefixInRoot,
    /// File uses `_unit.rs` suffix in tests root (should be in unit/).
    UnitSuffixInRoot,
    /// File uses `_integration.rs` suffix in tests root (should be in integration/).
    IntegrationSuffixInRoot,
    /// File uses `_py.rs` suffix suggesting Python binding tests.
    PySuffixInRoot,
    /// Scattered test file in root without proper categorization.
    ScatteredTestFile,
    /// Directory not in the allowed list.
    UnexpectedDirectory,
}

impl std::fmt::Display for ViolationKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::TestPrefixInRoot => write!(f, "test_ prefix in tests root"),
            Self::UnitSuffixInRoot => write!(f, "_unit.rs suffix in tests root"),
            Self::IntegrationSuffixInRoot => write!(f, "_integration.rs suffix in tests root"),
            Self::PySuffixInRoot => write!(f, "_py.rs suffix in tests root"),
            Self::ScatteredTestFile => write!(f, "scattered test file in root"),
            Self::UnexpectedDirectory => write!(f, "unexpected directory"),
        }
    }
}

/// Allowed directories in tests/ root.
const ALLOWED_DIRS: &[&str] = &[
    "scenarios",
    "snapshots",
    "fixtures",
    "support",
    "unit",
    "integration",
    "performance",
    "common",
];

/// Allowed root file names in tests/ (entry points and explicit gateways).
const ALLOWED_ROOT_FILE_PATTERNS: &[&str] = &[
    "mod.rs",
    "lib.rs",
    "scenarios_test.rs",
    "xiuxian-testing-gate.rs",
];

/// Check if a file name matches an allowed root file pattern.
fn is_allowed_root_file(name: &str, policy: Option<&TestsStructurePolicy>) -> bool {
    // Allow explicit root file names and policy overrides only.
    ALLOWED_ROOT_FILE_PATTERNS.contains(&name)
        || policy.is_some_and(|config| config.allowed_root_files.iter().any(|entry| entry == name))
}

/// Check if a directory name is allowed directly under tests/.
fn is_allowed_directory(name: &str, policy: Option<&TestsStructurePolicy>) -> bool {
    ALLOWED_DIRS.contains(&name)
        || policy.is_some_and(|config| config.allowed_directories.iter().any(|entry| entry == name))
}

/// Check if a file name indicates it should be in unit/.
fn is_unit_test_file(name: &str) -> bool {
    name.ends_with("_unit.rs") || name.starts_with("unit_")
}

/// Check if a file name indicates it should be in integration/.
fn is_integration_test_file(name: &str) -> bool {
    name.ends_with("_integration.rs")
        || name.starts_with("integration_")
        || name.contains("_indexer_")
        || name.contains("_debug")
}

/// Validate the structure of a tests/ directory.
///
/// # Arguments
///
/// * `tests_dir` - Path to the tests/ directory to validate.
///
/// # Returns
///
/// A vector of violations found. Empty if the structure is valid.
///
/// # Example
///
/// ```
/// use xiuxian_testing::validation::validate_tests_structure;
/// use std::path::Path;
///
/// let violations = validate_tests_structure(Path::new("tests"));
/// for v in &violations {
///     println!("{}: {} - {}", v.path.display(), v.kind, v.suggestion);
/// }
/// ```
#[must_use]
pub fn validate_tests_structure(tests_dir: &Path) -> Vec<StructureViolation> {
    validate_tests_structure_with_policy(tests_dir, None)
}

/// Validate the structure of a tests/ directory with optional policy overrides.
#[must_use]
pub fn validate_tests_structure_with_policy(
    tests_dir: &Path,
    policy: Option<&TestsStructurePolicy>,
) -> Vec<StructureViolation> {
    let mut violations = Vec::new();

    if !tests_dir.exists() {
        return violations;
    }

    let Ok(entries) = fs::read_dir(tests_dir) else {
        return violations;
    };

    for entry in entries.flatten() {
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().to_string();

        if path.is_dir() {
            // Check if directory is allowed
            if !is_allowed_directory(&name, policy) {
                violations.push(StructureViolation {
                    path: path.clone(),
                    kind: ViolationKind::UnexpectedDirectory,
                    suggestion: format!(
                        "Consider moving '{name}' to a standard location or allow it via \
                         tests/xiuxian-testings-rules.toml [tests].allowed_directories"
                    ),
                });
            }
        } else if Path::new(&name)
            .extension()
            .is_some_and(|ext| ext.eq_ignore_ascii_case("rs"))
        {
            // Check .rs file naming

            // Skip allowed root files
            if is_allowed_root_file(&name, policy) {
                continue;
            }

            // Check for test_ prefix (should be in unit/ or integration/)
            if name.starts_with("test_") {
                let base_name = name.strip_prefix("test_").unwrap_or(&name);
                let category = if is_integration_test_file(&name) {
                    "integration"
                } else {
                    "unit"
                };
                let suggested_name = base_name.replace("_integration", "").replace("_unit", "");
                violations.push(StructureViolation {
                    path: path.clone(),
                    kind: ViolationKind::TestPrefixInRoot,
                    suggestion: format!("Move to tests/{category}/{suggested_name}.rs"),
                });
                continue;
            }

            // Check for unit test file patterns
            if is_unit_test_file(&name) {
                let base_name = name
                    .strip_suffix("_unit.rs")
                    .or_else(|| name.strip_prefix("unit_"))
                    .unwrap_or(&name)
                    .strip_suffix(".rs")
                    .unwrap_or(&name);
                violations.push(StructureViolation {
                    path: path.clone(),
                    kind: ViolationKind::UnitSuffixInRoot,
                    suggestion: format!("Move to tests/unit/{base_name}.rs"),
                });
                continue;
            }

            // Check for _integration.rs suffix
            if name.ends_with("_integration.rs") {
                let base_name = name.strip_suffix("_integration.rs").unwrap_or(&name);
                violations.push(StructureViolation {
                    path: path.clone(),
                    kind: ViolationKind::IntegrationSuffixInRoot,
                    suggestion: format!("Move to tests/integration/{base_name}.rs"),
                });
                continue;
            }

            // Check for _py.rs suffix (Python binding tests)
            if name.ends_with("_py.rs") {
                let base_name = name.strip_suffix("_py.rs").unwrap_or(&name);
                violations.push(StructureViolation {
                    path: path.clone(),
                    kind: ViolationKind::PySuffixInRoot,
                    suggestion: format!(
                        "Move to tests/integration/{base_name}_python.rs or tests/unit/{base_name}_python.rs"
                    ),
                });
                continue;
            }

            // Any other root-level Rust test file should be organized or explicitly allowed.
            violations.push(StructureViolation {
                path: path.clone(),
                kind: ViolationKind::ScatteredTestFile,
                suggestion: "Move to tests/unit/ or tests/integration/ based on test scope, or \
                             allow this file via tests/xiuxian-testings-rules.toml [tests].allowed_root_files"
                    .to_string(),
            });
        }
    }

    violations
}

/// Validate tests structure for a specific crate.
///
/// # Arguments
///
/// * `crate_path` - Path to the crate root (containing Cargo.toml).
///
/// # Returns
///
/// A vector of violations found in the crate's tests/ directory.
#[must_use]
pub fn validate_crate_tests(crate_path: &Path) -> Vec<StructureViolation> {
    validate_crate_tests_with_policy(crate_path, None)
}

/// Validate tests structure for a specific crate with optional policy overrides.
#[must_use]
pub fn validate_crate_tests_with_policy(
    crate_path: &Path,
    policy: Option<&TestsStructurePolicy>,
) -> Vec<StructureViolation> {
    validate_tests_structure_with_policy(&crate_path.join("tests"), policy)
}

/// Get a summary report of violations.
///
/// # Arguments
///
/// * `violations` - List of violations to summarize.
///
/// # Returns
///
/// A human-readable summary string.
#[must_use]
pub fn format_violation_report(violations: &[StructureViolation]) -> String {
    use std::fmt::Write;

    if violations.is_empty() {
        return "✅ No violations found. Tests structure follows conventions.".to_string();
    }

    let mut report = String::new();
    let _ = write!(
        report,
        "❌ Found {} test structure violation(s):\n\n",
        violations.len()
    );

    for (i, v) in violations.iter().enumerate() {
        let _ = write!(
            report,
            "{}. {} ({})\n   💡 {}\n\n",
            i + 1,
            v.path.display(),
            v.kind,
            v.suggestion
        );
    }

    report.push_str(
        "\n📖 See: packages/rust/crates/xiuxian-testing/src/validation.rs for conventions\n",
    );

    report
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_is_allowed_root_file() {
        assert!(is_allowed_root_file("mod.rs", None));
        assert!(is_allowed_root_file("scenarios_test.rs", None));
        assert!(is_allowed_root_file("xiuxian-testing-gate.rs", None));
        assert!(!is_allowed_root_file("my_test.rs", None));
        assert!(!is_allowed_root_file("test_entity.rs", None));
        assert!(!is_allowed_root_file("entity_unit.rs", None));
    }

    #[test]
    fn test_is_allowed_root_file_with_policy_override() {
        let policy = TestsStructurePolicy {
            allowed_directories: Vec::new(),
            allowed_root_files: vec!["quantum_fusion_saliency_window.rs".to_string()],
        };
        assert!(is_allowed_root_file(
            "quantum_fusion_saliency_window.rs",
            Some(&policy)
        ));
    }

    #[test]
    fn test_is_unit_test_file() {
        assert!(is_unit_test_file("entity_unit.rs"));
        assert!(is_unit_test_file("unit_storage.rs"));
        assert!(!is_unit_test_file("entity.rs"));
        assert!(!is_unit_test_file("test_entity.rs"));
    }

    #[test]
    fn test_is_integration_test_file() {
        assert!(is_integration_test_file(
            "dependency_indexer_integration.rs"
        ));
        assert!(!is_integration_test_file("entity_unit.rs"));
    }

    #[test]
    fn test_performance_directory_is_allowed_by_default() {
        assert!(is_allowed_directory("performance", None));
    }

    #[test]
    fn test_validate_nonexistent_directory() {
        let violations = validate_tests_structure(Path::new("/nonexistent/path/tests"));
        assert!(violations.is_empty());
    }

    #[test]
    fn test_validate_tests_structure_flags_unclassified_root_rs_file() {
        let temp =
            tempfile::tempdir().unwrap_or_else(|error| panic!("tempdir should exist: {error}"));
        let tests_dir = temp.path().join("tests");
        fs::create_dir_all(&tests_dir)
            .unwrap_or_else(|error| panic!("tests dir should exist: {error}"));
        fs::write(
            tests_dir.join("quantum_fusion_saliency_window.rs"),
            "#[test]\nfn smoke() {}\n",
        )
        .unwrap_or_else(|error| panic!("test fixture should exist: {error}"));

        let violations = validate_tests_structure(&tests_dir);
        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].kind, ViolationKind::ScatteredTestFile);
    }

    #[test]
    fn test_validate_tests_structure_with_policy_allows_root_file_and_directory() {
        let temp =
            tempfile::tempdir().unwrap_or_else(|error| panic!("tempdir should exist: {error}"));
        let tests_dir = temp.path().join("tests");
        fs::create_dir_all(tests_dir.join("bench"))
            .unwrap_or_else(|error| panic!("custom directory should exist: {error}"));
        fs::write(
            tests_dir.join("coactivation_multihop_diffusion.rs"),
            "#[test]\nfn smoke() {}\n",
        )
        .unwrap_or_else(|error| panic!("test fixture should exist: {error}"));

        let policy = TestsStructurePolicy {
            allowed_directories: vec!["bench".to_string()],
            allowed_root_files: vec!["coactivation_multihop_diffusion.rs".to_string()],
        };
        let violations = validate_tests_structure_with_policy(&tests_dir, Some(&policy));
        assert!(
            violations.is_empty(),
            "expected no violations: {violations:?}"
        );
    }

    #[test]
    fn test_format_violation_report_empty() {
        let report = format_violation_report(&[]);
        assert!(report.contains("No violations"));
    }

    #[test]
    fn test_format_violation_report_with_violations() {
        let violations = vec![StructureViolation {
            path: PathBuf::from("tests/test_entity.rs"),
            kind: ViolationKind::TestPrefixInRoot,
            suggestion: "Move to tests/unit/entity.rs".to_string(),
        }];
        let report = format_violation_report(&violations);
        assert!(report.contains("Found 1"));
        assert!(report.contains("test_entity.rs"));
        assert!(report.contains("Move to tests/unit/entity.rs"));
    }
}
