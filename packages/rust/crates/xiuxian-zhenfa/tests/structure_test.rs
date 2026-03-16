//! Validate xiuxian-zhenfa tests structure.

use std::path::Path;

use xiuxian_testing::validation::{format_violation_report, validate_crate_tests};

#[test]
fn zhenfa_tests_structure_is_valid() {
    let crate_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let violations = validate_crate_tests(crate_root);

    assert!(
        violations.is_empty(),
        "{}",
        format_violation_report(&violations)
    );
}
