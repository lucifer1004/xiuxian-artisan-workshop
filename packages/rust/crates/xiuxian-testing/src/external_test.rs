//! External test module convention support.
//!
//! This module provides utilities for the "external test module" pattern,
//! where inline `#[cfg(test)]` modules are mounted from external files
//! using `#[path]` attribute.
//!
//! # Pattern Overview
//!
//! Instead of embedding tests inline (which bloats source files):
//!
//! ```ignore
//! // src/foo/bar.rs (BAD - tests inline, file grows)
//! fn business_logic() { ... }
//!
//! #[cfg(test)]
//! mod tests {
//!     // 500 lines of test code...
//! }
//! ```
//!
//! Use external mounting:
//!
//! ```ignore
//! // src/foo/bar.rs (GOOD - clean separation)
//! fn business_logic() { ... }
//!
//! #[cfg(test)]
//! #[path = "../../tests/unit/foo/bar.rs"]
//! mod tests;
//! ```
//!
//! # Directory Convention
//!
//! ```text
//! src/
//! └── foo/
//!     └── bar.rs              # Source file
//! tests/
//! └── unit/
//!     └── foo/
//!         └── bar.rs          # Corresponding test file (same path)
//! ```
//!
//! # Path Calculation
//!
//! | Source Location | Test Path |
//! |----------------|-----------|
//! | `src/foo.rs` | `tests/unit/foo.rs` |
//! | `src/foo/bar.rs` | `tests/unit/foo/bar.rs` |
//! | `src/foo/bar/baz.rs` | `tests/unit/foo/bar/baz.rs` |
//!
//! # test_api Pattern
//!
//! To test private functions, expose them via `test_api` module:
//!
//! ```ignore
//! // src/foo/bar.rs
//! fn internal_helper() { ... }  // Private
//!
//! #[cfg(test)]
//! pub mod test_api {
//!     pub use super::internal_helper;
//! }
//!
//! #[cfg(test)]
//! #[path = "../../tests/unit/foo/bar.rs"]
//! mod tests;
//! ```
//!
//! ```ignore
//! // tests/unit/foo/bar.rs
//! use crate::foo::bar::test_api::*;
//!
//! #[test]
//! fn test_internal_helper() {
//!     internal_helper();  // Accessible via test_api
//! }
//! ```

use std::path::{Path, PathBuf};

/// Calculate the external test path for a source file.
///
/// # Arguments
///
/// * `source_path` - Path to the source file (relative to crate root or absolute)
/// * `tests_root` - Path to the tests directory (usually "tests")
///
/// # Returns
///
/// The expected path to the external test file.
///
/// # Example
///
/// ```
/// use xiuxian_testing::external_test::calculate_test_path;
/// use std::path::{Path, PathBuf};
///
/// let source = Path::new("src/foo/bar.rs");
/// let test_path = calculate_test_path(source, "tests");
/// assert_eq!(test_path, PathBuf::from("tests/unit/foo/bar.rs"));
/// ```
pub fn calculate_test_path(source_path: &Path, tests_root: &str) -> PathBuf {
    let mut result = PathBuf::from(tests_root);
    result.push("unit");

    // Strip "src/" prefix if present
    let relative = source_path.strip_prefix("src").unwrap_or(source_path);

    result.push(relative);
    result
}

/// Generate the `#[path]` attribute value for mounting external tests.
///
/// # Arguments
///
/// * `source_path` - Path to the source file (relative to crate root)
/// * `tests_root` - Path to the tests directory (usually "tests")
///
/// # Returns
///
/// The relative path to use in `#[path = "..."]`.
///
/// # Example
///
/// ```
/// use xiuxian_testing::external_test::generate_path_attribute;
/// use std::path::Path;
///
/// // For src/foo/bar.rs, calculate path to tests/unit/foo/bar.rs
/// let path_attr = generate_path_attribute(Path::new("src/foo/bar.rs"), "tests");
/// assert_eq!(path_attr, "../../tests/unit/foo/bar.rs");
/// ```
pub fn generate_path_attribute(source_path: &Path, tests_root: &str) -> String {
    let test_path = calculate_test_path(source_path, tests_root);

    // Count directory depth from source file to crate root
    let depth = source_path
        .strip_prefix("src")
        .unwrap_or(source_path)
        .components()
        .count();

    // Build relative path: each level needs "../"
    let mut prefix = String::new();
    for _ in 0..depth {
        prefix.push_str("../");
    }

    format!("{}{}", prefix, test_path.display())
}

/// Information about an external test mounting point.
#[derive(Debug, Clone)]
pub struct ExternalTestMount {
    /// Source file that mounts the test.
    pub source_file: PathBuf,
    /// External test file path.
    pub test_file: PathBuf,
    /// The `#[path]` attribute value.
    pub path_attribute: String,
    /// Whether the test file exists.
    pub test_file_exists: bool,
}

/// Validate external test mountings in a crate.
///
/// This function scans source files for `#[path]` attributes pointing to
/// external test files and validates that:
/// 1. The referenced test file exists
/// 2. The path follows the convention (tests/unit/...)
///
/// # Arguments
///
/// * `crate_root` - Path to the crate root (containing Cargo.toml)
///
/// # Returns
///
/// A list of validation issues found.
pub fn validate_external_test_mounts(crate_root: &Path) -> Vec<ExternalTestValidationIssue> {
    let mut issues = Vec::new();
    let src_dir = crate_root.join("src");

    if !src_dir.exists() {
        return issues;
    }

    // Scan for files with #[path] attributes pointing to tests/
    if let Ok(entries) = std::fs::read_dir(&src_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                validate_external_tests_in_dir(&path, crate_root, &mut issues);
            } else if path.extension().map_or(false, |e| e == "rs") {
                validate_external_test_in_file(&path, crate_root, &mut issues);
            }
        }
    }

    issues
}

fn validate_external_tests_in_dir(
    dir: &Path,
    crate_root: &Path,
    issues: &mut Vec<ExternalTestValidationIssue>,
) {
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                validate_external_tests_in_dir(&path, crate_root, issues);
            } else if path.extension().map_or(false, |e| e == "rs") {
                validate_external_test_in_file(&path, crate_root, issues);
            }
        }
    }
}

fn validate_external_test_in_file(
    file_path: &Path,
    crate_root: &Path,
    issues: &mut Vec<ExternalTestValidationIssue>,
) {
    let content = match std::fs::read_to_string(file_path) {
        Ok(c) => c,
        Err(_) => return,
    };

    // Look for #[path = "..."] patterns
    for (line_num, line) in content.lines().enumerate() {
        if let Some(path_start) = line.find("#[path = \"") {
            if let Some(path_end) = line[path_start + 10..].find("\"]") {
                let path_value = &line[path_start + 10..path_start + 10 + path_end];

                // Check if it points to tests/
                if path_value.contains("tests/") {
                    let test_path = crate_root.join(path_value);

                    if !test_path.exists() {
                        issues.push(ExternalTestValidationIssue::MissingTestFile {
                            source_file: file_path.to_path_buf(),
                            line_number: line_num + 1,
                            referenced_path: path_value.to_string(),
                        });
                    } else if !path_value.contains("tests/unit/")
                        && !path_value.contains("tests/integration/")
                    {
                        issues.push(ExternalTestValidationIssue::NonStandardPath {
                            source_file: file_path.to_path_buf(),
                            line_number: line_num + 1,
                            referenced_path: path_value.to_string(),
                            suggestion: format!(
                                "Consider using tests/unit/ or tests/integration/ instead of {}",
                                path_value
                            ),
                        });
                    }
                }
            }
        }
    }
}

/// Validation issue for external test mountings.
#[derive(Debug, Clone)]
pub enum ExternalTestValidationIssue {
    /// The referenced test file does not exist.
    MissingTestFile {
        /// Source file containing the #[path] attribute.
        source_file: PathBuf,
        /// Line number of the #[path] attribute.
        line_number: usize,
        /// The path referenced in #[path = "..."].
        referenced_path: String,
    },
    /// The path does not follow the convention.
    NonStandardPath {
        /// Source file containing the #[path] attribute.
        source_file: PathBuf,
        /// Line number of the #[path] attribute.
        line_number: usize,
        /// The path referenced in #[path = "..."].
        referenced_path: String,
        /// Suggested fix for the path.
        suggestion: String,
    },
    /// Inline test block found (should be externalized).
    InlineTestBlock {
        /// Source file containing the inline tests.
        source_file: PathBuf,
        /// Line number of the #[cfg(test)] attribute.
        line_number: usize,
        /// Number of lines in the test block.
        block_size: usize,
        /// Suggested external test file path.
        suggested_path: String,
    },
}

impl ExternalTestValidationIssue {
    /// Get a human-readable description.
    pub fn description(&self) -> String {
        match self {
            Self::MissingTestFile {
                source_file,
                line_number,
                referenced_path,
            } => format!(
                "{}:{}: Referenced test file '{}' does not exist",
                source_file.display(),
                line_number,
                referenced_path
            ),
            Self::NonStandardPath {
                source_file,
                line_number,
                referenced_path,
                suggestion,
            } => format!(
                "{}:{}: Non-standard test path '{}'. {}",
                source_file.display(),
                line_number,
                referenced_path,
                suggestion
            ),
            Self::InlineTestBlock {
                source_file,
                line_number,
                block_size,
                suggested_path,
            } => format!(
                "{}:{}: Inline test block ({} lines) should be externalized to '{}'",
                source_file.display(),
                line_number,
                block_size,
                suggested_path
            ),
        }
    }
}

/// Detect inline test blocks in a crate's source files.
///
/// This function scans for `#[cfg(test)] mod tests { ... }` patterns
/// where the test module body is inline (not external via `#[path]`).
///
/// # Arguments
///
/// * `crate_root` - Path to the crate root (containing Cargo.toml)
/// * `min_block_lines` - Minimum lines to consider a block "large" (default: 10)
///
/// # Returns
///
/// A list of inline test block issues found.
pub fn detect_inline_test_blocks(
    crate_root: &Path,
    min_block_lines: usize,
) -> Vec<ExternalTestValidationIssue> {
    let mut issues = Vec::new();
    let src_dir = crate_root.join("src");

    if !src_dir.exists() {
        return issues;
    }

    scan_for_inline_tests(&src_dir, crate_root, min_block_lines, &mut issues);
    issues
}

fn scan_for_inline_tests(
    dir: &Path,
    crate_root: &Path,
    min_block_lines: usize,
    issues: &mut Vec<ExternalTestValidationIssue>,
) {
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                scan_for_inline_tests(&path, crate_root, min_block_lines, issues);
            } else if path.extension().map_or(false, |e| e == "rs") {
                detect_inline_test_in_file(&path, crate_root, min_block_lines, issues);
            }
        }
    }
}

fn detect_inline_test_in_file(
    file_path: &Path,
    crate_root: &Path,
    min_block_lines: usize,
    issues: &mut Vec<ExternalTestValidationIssue>,
) {
    let content = match std::fs::read_to_string(file_path) {
        Ok(c) => c,
        Err(_) => return,
    };

    let lines: Vec<&str> = content.lines().collect();
    let mut i = 0;

    while i < lines.len() {
        let line = lines[i].trim();

        // Look for #[cfg(test)] followed by mod tests {
        if line == "#[cfg(test)]"
            || line.starts_with("#[cfg(test,")
            || line.starts_with("#[cfg(all(test,")
        {
            // Check next lines for mod tests {
            let mut j = i + 1;
            while j < lines.len() && lines[j].trim().starts_with("#[") {
                j += 1; // Skip attributes like #[path]
            }

            if j < lines.len() {
                let mod_line = lines[j].trim();

                // Check if it's a test module declaration
                if mod_line.starts_with("mod tests") {
                    // Check if it has #[path] attribute (external test)
                    let has_path_attr = (i..j).any(|k| lines[k].contains("#[path ="));

                    if !has_path_attr {
                        // This is an inline test module
                        // Find the opening brace
                        let brace_line = if mod_line.ends_with('{') { j } else { j + 1 };

                        // Count lines in the block
                        let block_size = count_block_lines(&lines, brace_line);

                        if block_size >= min_block_lines {
                            // Generate suggested external path
                            let relative = file_path.strip_prefix(crate_root).unwrap_or(file_path);
                            let suggested = generate_path_attribute(relative, "tests");

                            issues.push(ExternalTestValidationIssue::InlineTestBlock {
                                source_file: file_path.to_path_buf(),
                                line_number: i + 1,
                                block_size,
                                suggested_path: suggested,
                            });
                        }
                    }
                }
            }
        }

        i += 1;
    }
}

/// Count the number of lines in a code block (from opening brace to matching closing brace).
fn count_block_lines(lines: &[&str], start_line: usize) -> usize {
    let mut depth = 0;
    let mut found_opening = false;
    let mut count = 0;

    for i in start_line..lines.len() {
        for ch in lines[i].chars() {
            if ch == '{' {
                depth += 1;
                found_opening = true;
            } else if ch == '}' {
                depth -= 1;
            }
        }
        count += 1;

        if found_opening && depth == 0 {
            break;
        }
    }

    count
}

/// Format a report of inline test block issues.
///
/// # Arguments
///
/// * `issues` - List of inline test block issues.
///
/// # Returns
///
/// A human-readable report string.
pub fn format_inline_test_report(issues: &[ExternalTestValidationIssue]) -> String {
    use std::fmt::Write;

    let inline_issues: Vec<_> = issues
        .iter()
        .filter(|i| matches!(i, ExternalTestValidationIssue::InlineTestBlock { .. }))
        .collect();

    if inline_issues.is_empty() {
        return "✅ No inline test blocks found. All tests properly externalized.".into();
    }

    let mut report = String::new();
    let cwd = std::env::current_dir().unwrap_or_default();

    writeln!(
        report,
        r#"❌ Found {} inline test block(s) that must be externalized.

================================================================================
                        INLINE TEST BLOCK VIOLATIONS
================================================================================
"#,
        inline_issues.len()
    )
    .unwrap();

    for (i, issue) in inline_issues.iter().enumerate() {
        if let ExternalTestValidationIssue::InlineTestBlock {
            source_file,
            line_number,
            block_size,
            suggested_path,
        } = issue
        {
            let relative = source_file
                .strip_prefix(&cwd)
                .unwrap_or(source_file)
                .to_string_lossy();
            let src_relative = relative
                .split_once("/src/")
                .map(|(_, s)| s)
                .unwrap_or(&relative);

            writeln!(
                report,
                r#"{i}. src/{src}
   📍 Line {line} | {size} lines of inline tests

   ✏️  HOW TO FIX:
   1. Create: tests/unit/{src}
   2. Cut the #[cfg(test)] mod tests {{ ... }} block from source
   3. Paste into the new test file (remove #[cfg(test)] wrapper)
   4. In source file, add at the end:

      #[cfg(test)]
      #[path = "{path}"]
      mod tests;

"#,
                i = i + 1,
                src = src_relative,
                line = line_number,
                size = block_size,
                path = suggested_path
            )
            .unwrap();
        }
    }

    writeln!(
        report,
        r#"================================================================================
                           DIRECTORY STRUCTURE
================================================================================

  src/foo/bar.rs              →  tests/unit/foo/bar.rs
  src/link_graph/parser.rs    →  tests/unit/link_graph/parser.rs

The #[path] attribute calculates relative path from source file to test file.
"#
    )
    .unwrap();

    report
}

#[cfg(test)]
mod tests {
    use super::*;

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
        // 3 levels deep = ../../../
        assert_eq!(path_attr, "../../../tests/unit/foo/bar/baz.rs");
    }
}
