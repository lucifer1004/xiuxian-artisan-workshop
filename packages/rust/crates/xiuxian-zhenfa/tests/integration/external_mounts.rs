//! Validate external test mounts in xiuxian-zhenfa.

use std::path::Path;
use xiuxian_testing::external_test::validate_external_test_mounts;

#[test]
fn external_test_mounts_are_valid() {
    let crate_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let issues = validate_external_test_mounts(crate_root);

    if !issues.is_empty() {
        for issue in &issues {
            eprintln!("{}", issue.description());
        }
    }

    assert!(issues.is_empty(), "invalid external test mounts detected");
}
