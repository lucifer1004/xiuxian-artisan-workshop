//! Focused integration coverage for the built-in `modularity` rule pack.

use std::fs;
use std::path::{Path, PathBuf};

use tempfile::TempDir;
use xiuxian_testing::{CollectionContext, ModularityRulePack, RulePack};

fn must_ok<T, E: std::fmt::Display>(result: Result<T, E>, context: &str) -> T {
    result.unwrap_or_else(|error| panic!("{context}: {error}"))
}

fn crate_src_root(temp_dir: &TempDir, crate_name: &str) -> PathBuf {
    temp_dir
        .path()
        .join("packages")
        .join("rust")
        .join("crates")
        .join(crate_name)
        .join("src")
}

fn write_rust_file(src_root: &Path, relative_path: &str, content: &str) {
    let path = src_root.join(relative_path);
    let parent = path
        .parent()
        .unwrap_or_else(|| panic!("target file should have parent: {}", path.display()));
    must_ok(
        fs::create_dir_all(parent),
        "should create parent directories for fixture file",
    );
    must_ok(
        fs::write(&path, content),
        "should write fixture rust source file",
    );
}

fn evaluate_fixture(crate_name: &str, temp_dir: &TempDir) -> Vec<xiuxian_testing::ContractFinding> {
    let ctx = CollectionContext {
        suite_id: "contracts".to_string(),
        crate_name: Some(crate_name.to_string()),
        workspace_root: Some(temp_dir.path().to_path_buf()),
        labels: std::collections::BTreeMap::new(),
    };

    let pack = ModularityRulePack;
    let artifacts = must_ok(pack.collect(&ctx), "modularity collect should succeed");
    must_ok(
        pack.evaluate(&artifacts),
        "modularity evaluation should succeed",
    )
}

#[test]
fn modularity_pack_flags_mod_rs_with_implementation_logic() {
    let temp_dir = must_ok(TempDir::new(), "should create temp dir");
    let src_root = crate_src_root(&temp_dir, "demo");
    write_rust_file(
        &src_root,
        "feature/mod.rs",
        r"
mod parser;
pub use parser::Parser;

pub fn parse() {}
",
    );
    write_rust_file(&src_root, "feature/parser.rs", "pub(crate) struct Parser;");

    let findings = evaluate_fixture("demo", &temp_dir);
    assert!(findings.iter().any(|finding| finding.rule_id == "MOD-R001"));
}

#[test]
fn modularity_pack_flags_public_result_without_errors_doc() {
    let temp_dir = must_ok(TempDir::new(), "should create temp dir");
    let src_root = crate_src_root(&temp_dir, "demo");
    write_rust_file(
        &src_root,
        "api.rs",
        r"
pub fn run() -> anyhow::Result<()> {
    Ok(())
}
",
    );

    let findings = evaluate_fixture("demo", &temp_dir);
    assert!(findings.iter().any(|finding| finding.rule_id == "MOD-R003"));
}

#[test]
fn modularity_pack_flags_overly_broad_visibility_in_internal_module() {
    let temp_dir = must_ok(TempDir::new(), "should create temp dir");
    let src_root = crate_src_root(&temp_dir, "demo");
    write_rust_file(
        &src_root,
        "internal/state.rs",
        r"
pub struct InternalState {
    value: usize,
}
",
    );

    let findings = evaluate_fixture("demo", &temp_dir);
    assert!(findings.iter().any(|finding| finding.rule_id == "MOD-R002"));
}

#[test]
fn modularity_pack_accepts_interface_only_and_documented_error_contracts() {
    let temp_dir = must_ok(TempDir::new(), "should create temp dir");
    let src_root = crate_src_root(&temp_dir, "demo");
    write_rust_file(
        &src_root,
        "feature/mod.rs",
        r"
mod parser;
pub(crate) use parser::Parser;
",
    );
    write_rust_file(
        &src_root,
        "api.rs",
        r"
/// Execute the operation.
///
/// # Errors
/// Returns an error when upstream resolution fails.
pub fn run() -> anyhow::Result<()> {
    Ok(())
}
",
    );
    write_rust_file(
        &src_root,
        "internal/state.rs",
        r"
pub(crate) struct InternalState {
    value: usize,
}
",
    );

    let findings = evaluate_fixture("demo", &temp_dir);
    assert!(findings.is_empty());
}
