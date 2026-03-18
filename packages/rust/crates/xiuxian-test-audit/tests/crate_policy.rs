//! Validate shared xiuxian test policy without compiling consumer crates.

use std::path::{Path, PathBuf};

use xiuxian_testing::assert_crate_test_policy;

fn workspace_root() -> PathBuf {
    let workspace_root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../../");
    match workspace_root.canonicalize() {
        Ok(path) => path,
        Err(error) => panic!("workspace root should exist: {error}"),
    }
}

fn crate_root(crate_name: &str) -> PathBuf {
    workspace_root()
        .join("packages/rust/crates")
        .join(crate_name)
}

#[test]
fn zhenfa_test_policy_is_valid() {
    assert_crate_test_policy(&crate_root("xiuxian-zhenfa"));
}

#[test]
fn wendao_test_policy_is_valid() {
    assert_crate_test_policy(&crate_root("xiuxian-wendao"));
}
