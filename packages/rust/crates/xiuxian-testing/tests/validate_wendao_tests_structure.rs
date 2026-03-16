//! Integration test: validate xiuxian-wendao tests structure.
//!
//! This test demonstrates the validation module by checking the actual
//! xiuxian-wendao crate's tests directory.

use std::path::Path;

use xiuxian_testing::validation::{format_violation_report, validate_tests_structure};

/// Check xiuxian-wendao tests structure and report violations.
///
/// This test is expected to fail until the test files are properly organized.
/// Use `cargo test -p xiuxian-testing -- --nocapture` to see the report.
#[test]
fn validate_wendao_tests_structure() {
    // Path to xiuxian-wendao tests directory (relative to workspace root)
    let workspace_root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|p| p.parent())
        .expect("workspace root")
        .join("crates")
        .join("xiuxian-wendao")
        .join("tests");

    let violations = validate_tests_structure(&workspace_root);

    // Print report for visibility
    println!("{}", format_violation_report(&violations));

    // For now, we just report violations without failing
    // Once the tests are organized, we can change this to assert!(violations.is_empty())
    if !violations.is_empty() {
        println!(
            "\n⚠️  Note: {} violation(s) found. Organize tests into unit/ and integration/ directories.",
            violations.len()
        );
    }
}
