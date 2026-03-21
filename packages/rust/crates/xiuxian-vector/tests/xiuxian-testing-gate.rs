//! Test-structure policy gate for xiuxian-vector.

use std::path::Path;

use xiuxian_testing::assert_crate_tests_structure_with_workspace_config;

#[test]
fn enforce_tests_structure_gate() {
    assert_crate_tests_structure_with_workspace_config(Path::new(env!("CARGO_MANIFEST_DIR")));
}
