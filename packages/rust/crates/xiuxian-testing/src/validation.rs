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
//! └── scenarios_test.rs    # Scenario test entry point
//! ```
//!
//! # Naming Conventions
//!
//! - **Unit tests**: `tests/unit/{module}.rs` (e.g., `entity.rs`, `storage.rs`)
//! - **Integration tests**: `tests/integration/{feature}.rs`
//! - **Test entry points**: `tests/{name}_test.rs` (e.g., `scenarios_test.rs`)
//!
//! # Forbidden Patterns
//!
//! - `tests/test_*.rs` → Move to `tests/unit/` or `tests/integration/`
//! - `tests/*_unit.rs` → Move to `tests/unit/{name}.rs`
//! - `tests/*_integration.rs` → Move to `tests/integration/{name}.rs`
//! - Scattered files in `tests/` root → Organize into subdirectories

use std::fs;
use std::path::{Path, PathBuf};

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
    "common",
];

/// Allowed file patterns in tests/ root (entry points).
const ALLOWED_ROOT_FILE_PATTERNS: &[&str] = &["mod.rs", "lib.rs", "scenarios_test.rs"];

/// Check if a file name matches an allowed root file pattern.
fn is_allowed_root_file(name: &str) -> bool {
    // Allow entry points like *_test.rs
    if name.ends_with("_test.rs") {
        return true;
    }
    // Allow specific patterns
    ALLOWED_ROOT_FILE_PATTERNS.contains(&name)
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

/// Check if a file is a test file (not a helper module).
fn is_likely_test_file(name: &str) -> bool {
    let name_lower = name.to_lowercase();
    name_lower.contains("test")
        || name_lower.contains("_unit")
        || name_lower.contains("_integration")
        || name_lower.contains("_py")
        || name_lower.contains("_rpc")
        || name_lower.contains("_contracts")
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
pub fn validate_tests_structure(tests_dir: &Path) -> Vec<StructureViolation> {
    let mut violations = Vec::new();

    if !tests_dir.exists() {
        return violations;
    }

    let entries = match fs::read_dir(tests_dir) {
        Ok(e) => e,
        Err(_) => return violations,
    };

    for entry in entries.flatten() {
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().to_string();

        if path.is_dir() {
            // Check if directory is allowed
            if !ALLOWED_DIRS.contains(&name.as_str()) {
                violations.push(StructureViolation {
                    path: path.clone(),
                    kind: ViolationKind::UnexpectedDirectory,
                    suggestion: format!(
                        "Consider moving '{}' to a standard location or adding to ALLOWED_DIRS",
                        name
                    ),
                });
            }
        } else if name.ends_with(".rs") {
            // Check .rs file naming

            // Skip allowed root files
            if is_allowed_root_file(&name) {
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
                violations.push(StructureViolation {
                    path: path.clone(),
                    kind: ViolationKind::TestPrefixInRoot,
                    suggestion: format!(
                        "Move to tests/{}/{}.rs",
                        category,
                        base_name.replace("_integration", "").replace("_unit", "")
                    ),
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
                    suggestion: format!("Move to tests/unit/{}.rs", base_name),
                });
                continue;
            }

            // Check for _integration.rs suffix
            if name.ends_with("_integration.rs") {
                let base_name = name.strip_suffix("_integration.rs").unwrap_or(&name);
                violations.push(StructureViolation {
                    path: path.clone(),
                    kind: ViolationKind::IntegrationSuffixInRoot,
                    suggestion: format!("Move to tests/integration/{}.rs", base_name),
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
                        "Move to tests/integration/{}_python.rs or tests/unit/{}_python.rs",
                        base_name, base_name
                    ),
                });
                continue;
            }

            // Check for other scattered test files
            if is_likely_test_file(&name) && !is_allowed_root_file(&name) {
                violations.push(StructureViolation {
                    path: path.clone(),
                    kind: ViolationKind::ScatteredTestFile,
                    suggestion: format!(
                        "Move to tests/unit/ or tests/integration/ based on test scope"
                    ),
                });
            }
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
pub fn validate_crate_tests(crate_path: &Path) -> Vec<StructureViolation> {
    validate_tests_structure(&crate_path.join("tests"))
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
pub fn format_violation_report(violations: &[StructureViolation]) -> String {
    if violations.is_empty() {
        return "✅ No violations found. Tests structure follows conventions.".to_string();
    }

    let mut report = format!(
        "❌ Found {} test structure violation(s):\n\n",
        violations.len()
    );

    for (i, v) in violations.iter().enumerate() {
        report.push_str(&format!(
            "{}. {} ({})\n   💡 {}\n\n",
            i + 1,
            v.path.display(),
            v.kind,
            v.suggestion
        ));
    }

    report.push_str(
        "\n📖 See: packages/rust/crates/xiuxian-testing/src/validation.rs for conventions\n",
    );

    report
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_allowed_root_file() {
        assert!(is_allowed_root_file("mod.rs"));
        assert!(is_allowed_root_file("scenarios_test.rs"));
        assert!(is_allowed_root_file("my_test.rs"));
        assert!(!is_allowed_root_file("test_entity.rs"));
        assert!(!is_allowed_root_file("entity_unit.rs"));
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
    fn test_validate_nonexistent_directory() {
        let violations = validate_tests_structure(Path::new("/nonexistent/path/tests"));
        assert!(violations.is_empty());
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
