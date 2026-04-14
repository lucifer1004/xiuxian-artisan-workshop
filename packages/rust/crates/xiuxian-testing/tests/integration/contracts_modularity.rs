//! Focused integration coverage for the built-in `modularity` rule pack.

use std::fmt::Write as _;
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
fn modularity_pack_flags_inline_module_body_in_mod_rs() {
    let temp_dir = must_ok(TempDir::new(), "should create temp dir");
    let src_root = crate_src_root(&temp_dir, "demo");
    write_rust_file(
        &src_root,
        "feature/mod.rs",
        r"
mod parser {
    pub(crate) struct Parser;
}

pub use parser::Parser;
",
    );

    let findings = evaluate_fixture("demo", &temp_dir);
    let finding = findings
        .iter()
        .find(|finding| finding.rule_id == "MOD-R001")
        .unwrap_or_else(|| panic!("expected MOD-R001 finding, got {findings:#?}"));
    assert!(
        finding.summary.contains("inline module `parser`"),
        "expected inline-module summary, got {finding:#?}"
    );
}

#[test]
fn modularity_pack_flags_private_use_import_in_mod_rs() {
    let temp_dir = must_ok(TempDir::new(), "should create temp dir");
    let src_root = crate_src_root(&temp_dir, "demo");
    write_rust_file(
        &src_root,
        "feature/mod.rs",
        r"
mod parser;
use parser::Parser;
pub use parser::Parser as PublicParser;
",
    );
    write_rust_file(&src_root, "feature/parser.rs", "pub(crate) struct Parser;");

    let findings = evaluate_fixture("demo", &temp_dir);
    let finding = findings
        .iter()
        .find(|finding| finding.rule_id == "MOD-R001")
        .unwrap_or_else(|| panic!("expected MOD-R001 finding, got {findings:#?}"));
    assert!(
        finding.summary.contains("private `use` import"),
        "expected private-use summary, got {finding:#?}"
    );
}

#[test]
fn modularity_pack_flags_glob_reexport_in_mod_rs() {
    let temp_dir = must_ok(TempDir::new(), "should create temp dir");
    let src_root = crate_src_root(&temp_dir, "demo");
    write_rust_file(
        &src_root,
        "feature/mod.rs",
        r"
mod parser;
pub use parser::*;
",
    );
    write_rust_file(&src_root, "feature/parser.rs", "pub(crate) struct Parser;");

    let findings = evaluate_fixture("demo", &temp_dir);
    let finding = findings
        .iter()
        .find(|finding| finding.rule_id == "MOD-R001")
        .unwrap_or_else(|| panic!("expected MOD-R001 finding, got {findings:#?}"));
    assert!(
        finding.summary.contains("glob re-export"),
        "expected glob-reexport summary, got {finding:#?}"
    );
}

#[test]
fn modularity_pack_flags_public_module_declaration_in_mod_rs() {
    let temp_dir = must_ok(TempDir::new(), "should create temp dir");
    let src_root = crate_src_root(&temp_dir, "demo");
    write_rust_file(
        &src_root,
        "feature/mod.rs",
        r"
pub mod parser;
pub use parser::Parser;
",
    );
    write_rust_file(&src_root, "feature/parser.rs", "pub struct Parser;");

    let findings = evaluate_fixture("demo", &temp_dir);
    let finding = findings
        .iter()
        .find(|finding| finding.rule_id == "MOD-R001")
        .unwrap_or_else(|| panic!("expected MOD-R001 finding, got {findings:#?}"));
    assert!(
        finding
            .summary
            .contains("visible module declaration `parser`"),
        "expected public-module summary, got {finding:#?}"
    );
}

#[test]
fn modularity_pack_flags_restricted_visible_module_declaration_in_mod_rs() {
    let temp_dir = must_ok(TempDir::new(), "should create temp dir");
    let src_root = crate_src_root(&temp_dir, "demo");
    write_rust_file(
        &src_root,
        "feature/mod.rs",
        r"
pub(crate) mod parser;
pub(crate) use parser::Parser;
",
    );
    write_rust_file(&src_root, "feature/parser.rs", "pub(crate) struct Parser;");

    let findings = evaluate_fixture("demo", &temp_dir);
    let finding = findings
        .iter()
        .find(|finding| finding.rule_id == "MOD-R001")
        .unwrap_or_else(|| panic!("expected MOD-R001 finding, got {findings:#?}"));
    assert!(
        finding
            .summary
            .contains("visible module declaration `parser`"),
        "expected restricted-visible-module summary, got {finding:#?}"
    );
}

#[test]
fn modularity_pack_flags_unparseable_mod_rs() {
    let temp_dir = must_ok(TempDir::new(), "should create temp dir");
    let src_root = crate_src_root(&temp_dir, "demo");
    write_rust_file(
        &src_root,
        "feature/mod.rs",
        r"
mod parser;

pub fn parse( {
",
    );
    write_rust_file(&src_root, "feature/parser.rs", "pub(crate) struct Parser;");

    let findings = evaluate_fixture("demo", &temp_dir);
    let finding = findings
        .iter()
        .find(|finding| finding.rule_id == "MOD-R001")
        .unwrap_or_else(|| panic!("expected MOD-R001 parse finding, got {findings:#?}"));
    assert!(
        finding
            .summary
            .contains("could not be parsed as Rust syntax"),
        "expected parse-failure summary, got {finding:#?}"
    );
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
fn modularity_pack_flags_bloated_multi_responsibility_file() {
    let temp_dir = must_ok(TempDir::new(), "should create temp dir");
    let src_root = crate_src_root(&temp_dir, "demo");

    let mut content = String::from(
        "pub struct Planner {\n    value: usize,\n}\n\npub struct RuntimeState {\n    active: bool,\n}\n\npub enum Mode {\n    Fast,\n    Safe,\n}\n\npub const DEFAULT_LIMIT: usize = 32;\n\n",
    );
    for idx in 0..24 {
        write!(
            content,
            "pub fn helper_{idx}(input: usize) -> usize {{\n    let base = input + {idx};\n    let staged = base + DEFAULT_LIMIT;\n    let guarded = staged.saturating_add({idx});\n    guarded + 1\n}}\n\n"
        )
        .unwrap_or_else(|error| panic!("should append helper fixture body: {error}"));
    }
    for idx in 0..18 {
        write!(
            content,
            "impl Planner {{\n    pub fn stage_{idx}(&self) -> usize {{\n        let local = self.value + {idx};\n        let bounded = local + DEFAULT_LIMIT;\n        bounded.saturating_sub({idx})\n    }}\n}}\n\n"
        )
        .unwrap_or_else(|error| panic!("should append impl fixture body: {error}"));
    }

    write_rust_file(&src_root, "feature.rs", &content);

    let findings = evaluate_fixture("demo", &temp_dir);
    let finding = findings
        .iter()
        .find(|finding| finding.rule_id == "MOD-R006")
        .unwrap_or_else(|| panic!("expected MOD-R006 finding, got {findings:#?}"));
    assert!(
        finding
            .title
            .contains("appears too large for one ownership seam"),
        "expected file-bloat summary, got {finding:#?}"
    );
}

#[test]
fn modularity_pack_accepts_compact_single_responsibility_file() {
    let temp_dir = must_ok(TempDir::new(), "should create temp dir");
    let src_root = crate_src_root(&temp_dir, "demo");
    write_rust_file(
        &src_root,
        "service.rs",
        r"
pub struct Service {
    value: usize,
}

impl Service {
    pub fn new(value: usize) -> Self {
        Self { value }
    }

    pub fn value(&self) -> usize {
        self.value
    }
}
",
    );

    let findings = evaluate_fixture("demo", &temp_dir);
    assert!(
        findings.iter().all(|finding| finding.rule_id != "MOD-R006"),
        "expected no MOD-R006 finding, got {findings:#?}"
    );
}

#[test]
fn modularity_pack_flags_root_module_that_loses_toc_role() {
    let temp_dir = must_ok(TempDir::new(), "should create temp dir");
    let src_root = crate_src_root(&temp_dir, "demo");
    write_rust_file(
        &src_root,
        "feature.rs",
        r"
mod parser;
mod runtime;
mod service;

pub(crate) use parser::Parser;
pub(crate) use service::Service;

pub struct FeatureState {
    stage: usize,
}

impl FeatureState {
    pub fn new(stage: usize) -> Self {
        Self { stage }
    }
}

pub fn execute(state: &FeatureState) -> usize {
    let first = state.stage + 1;
    let second = first + 2;
    let third = second + 3;
    let fourth = third + 4;
    let fifth = fourth + 5;
    let sixth = fifth + 6;
    let seventh = sixth + 7;
    let eighth = seventh + 8;
    let ninth = eighth + 9;
    let tenth = ninth + 10;
    let eleventh = tenth + 11;
    let twelfth = eleventh + 12;
    let thirteenth = twelfth + 13;
    let fourteenth = thirteenth + 14;
    let fifteenth = fourteenth + 15;
    let sixteenth = fifteenth + 16;
    let seventeenth = sixteenth + 17;
    let eighteenth = seventeenth + 18;
    let nineteenth = eighteenth + 19;
    let twentieth = nineteenth + 20;
    let twenty_first = twentieth + 21;
    let twenty_second = twenty_first + 22;
    let twenty_third = twenty_second + 23;
    let twenty_fourth = twenty_third + 24;
    let twenty_fifth = twenty_fourth + 25;
    let twenty_sixth = twenty_fifth + 26;
    let twenty_seventh = twenty_sixth + 27;
    let twenty_eighth = twenty_seventh + 28;
    let twenty_ninth = twenty_eighth + 29;
    let thirtieth = twenty_ninth + 30;
    let thirty_first = thirtieth + 31;
    let thirty_second = thirty_first + 32;
    let thirty_third = thirty_second + 33;
    let thirty_fourth = thirty_third + 34;
    let thirty_fifth = thirty_fourth + 35;
    let thirty_sixth = thirty_fifth + 36;
    let thirty_seventh = thirty_sixth + 37;
    let thirty_eighth = thirty_seventh + 38;
    let thirty_ninth = thirty_eighth + 39;
    let fortieth = thirty_ninth + 40;
    fortieth
}
",
    );
    write_rust_file(&src_root, "feature/parser.rs", "pub(crate) struct Parser;");
    write_rust_file(
        &src_root,
        "feature/runtime.rs",
        "pub(crate) struct Runtime;",
    );
    write_rust_file(
        &src_root,
        "feature/service.rs",
        "pub(crate) struct Service;",
    );

    let findings = evaluate_fixture("demo", &temp_dir);
    let finding = findings
        .iter()
        .find(|finding| finding.rule_id == "MOD-R007")
        .unwrap_or_else(|| panic!("expected MOD-R007 finding, got {findings:#?}"));
    assert!(
        finding.title.contains("navigational table of contents"),
        "expected root-toc title, got {finding:#?}"
    );
}

#[test]
fn modularity_pack_accepts_clear_folder_root_toc() {
    let temp_dir = must_ok(TempDir::new(), "should create temp dir");
    let src_root = crate_src_root(&temp_dir, "demo");
    write_rust_file(
        &src_root,
        "feature.rs",
        r"
mod parser;
mod runtime;
mod service;

pub(crate) use parser::Parser;
pub(crate) use runtime::Runtime;
pub(crate) use service::Service;
",
    );
    write_rust_file(&src_root, "feature/parser.rs", "pub(crate) struct Parser;");
    write_rust_file(
        &src_root,
        "feature/runtime.rs",
        "pub(crate) struct Runtime;",
    );
    write_rust_file(
        &src_root,
        "feature/service.rs",
        "pub(crate) struct Service;",
    );

    let findings = evaluate_fixture("demo", &temp_dir);
    assert!(
        findings.iter().all(|finding| finding.rule_id != "MOD-R007"),
        "expected no MOD-R007 finding, got {findings:#?}"
    );
}

#[test]
fn modularity_pack_flags_noisy_root_facade_exports() {
    let temp_dir = must_ok(TempDir::new(), "should create temp dir");
    let src_root = crate_src_root(&temp_dir, "demo");
    write_rust_file(
        &src_root,
        "feature.rs",
        r"
mod parser;
mod runtime;
mod service;

pub use self::{
    parser::{ParseError, ParseMode, ParsePlan},
    runtime::{Runtime, RuntimeConfig, RuntimeHandle},
    service::{Service, ServiceRequest, ServiceResponse},
};
",
    );
    write_rust_file(&src_root, "feature/parser.rs", "pub(crate) struct Parser;");
    write_rust_file(
        &src_root,
        "feature/runtime.rs",
        "pub(crate) struct Runtime;",
    );
    write_rust_file(
        &src_root,
        "feature/service.rs",
        "pub(crate) struct Service;",
    );

    let findings = evaluate_fixture("demo", &temp_dir);
    let finding = findings
        .iter()
        .find(|finding| finding.rule_id == "MOD-R008")
        .unwrap_or_else(|| panic!("expected MOD-R008 finding, got {findings:#?}"));
    assert!(
        finding.title.contains("export surface"),
        "expected root-facade title, got {finding:#?}"
    );
}

#[test]
fn modularity_pack_accepts_curated_root_facade_exports() {
    let temp_dir = must_ok(TempDir::new(), "should create temp dir");
    let src_root = crate_src_root(&temp_dir, "demo");
    write_rust_file(
        &src_root,
        "feature.rs",
        r"
mod parser;
mod runtime;
mod service;

pub use self::parser::Parser;
pub use self::runtime::Runtime;
pub use self::service::Service;
",
    );
    write_rust_file(&src_root, "feature/parser.rs", "pub struct Parser;");
    write_rust_file(&src_root, "feature/runtime.rs", "pub struct Runtime;");
    write_rust_file(&src_root, "feature/service.rs", "pub struct Service;");

    let findings = evaluate_fixture("demo", &temp_dir);
    assert!(
        findings.iter().all(|finding| finding.rule_id != "MOD-R008"),
        "expected no MOD-R008 finding, got {findings:#?}"
    );
}

#[test]
fn modularity_pack_flags_multi_hop_relative_imports() {
    let temp_dir = must_ok(TempDir::new(), "should create temp dir");
    let src_root = crate_src_root(&temp_dir, "demo");
    write_rust_file(
        &src_root,
        "feature/worker.rs",
        r"
use super::super::shared::SharedState;

pub(crate) fn run(state: SharedState) -> usize {
    state.value()
}
",
    );
    write_rust_file(
        &src_root,
        "shared.rs",
        r"
pub(crate) struct SharedState(usize);

impl SharedState {
    pub(crate) fn value(&self) -> usize {
        self.0
    }
}
",
    );

    let findings = evaluate_fixture("demo", &temp_dir);
    let finding = findings
        .iter()
        .find(|finding| finding.rule_id == "MOD-R009")
        .unwrap_or_else(|| panic!("expected MOD-R009 finding, got {findings:#?}"));
    assert!(
        finding.title.contains("Prefer `crate::`"),
        "expected relative-import title, got {finding:#?}"
    );
}

#[test]
fn modularity_pack_accepts_crate_qualified_imports() {
    let temp_dir = must_ok(TempDir::new(), "should create temp dir");
    let src_root = crate_src_root(&temp_dir, "demo");
    write_rust_file(
        &src_root,
        "feature/worker.rs",
        r"
use crate::shared::SharedState;

pub(crate) fn run(state: SharedState) -> usize {
    state.value()
}
",
    );
    write_rust_file(
        &src_root,
        "shared.rs",
        r"
pub(crate) struct SharedState(usize);

impl SharedState {
    pub(crate) fn value(&self) -> usize {
        self.0
    }
}
",
    );

    let findings = evaluate_fixture("demo", &temp_dir);
    assert!(
        findings.iter().all(|finding| finding.rule_id != "MOD-R009"),
        "expected no MOD-R009 finding, got {findings:#?}"
    );
}

#[test]
fn modularity_pack_flags_public_alias_reexport_in_root_facade() {
    let temp_dir = must_ok(TempDir::new(), "should create temp dir");
    let src_root = crate_src_root(&temp_dir, "demo");
    write_rust_file(
        &src_root,
        "feature.rs",
        r"
mod parser;
mod runtime;
mod service;

pub use self::parser::Parser as FeatureParser;
pub use self::runtime::Runtime;
pub use self::service::Service;
",
    );
    write_rust_file(&src_root, "feature/parser.rs", "pub struct Parser;");
    write_rust_file(&src_root, "feature/runtime.rs", "pub struct Runtime;");
    write_rust_file(&src_root, "feature/service.rs", "pub struct Service;");

    let findings = evaluate_fixture("demo", &temp_dir);
    let finding = findings
        .iter()
        .find(|finding| finding.rule_id == "MOD-R010")
        .unwrap_or_else(|| panic!("expected MOD-R010 finding, got {findings:#?}"));
    assert!(
        finding.title.contains("alias re-exports"),
        "expected root-alias title, got {finding:#?}"
    );
}

#[test]
fn modularity_pack_accepts_restricted_alias_reexport_in_root_facade() {
    let temp_dir = must_ok(TempDir::new(), "should create temp dir");
    let src_root = crate_src_root(&temp_dir, "demo");
    write_rust_file(
        &src_root,
        "feature.rs",
        r"
mod parser;
mod runtime;
mod service;

pub(crate) use self::parser::Parser as InternalParser;
pub(crate) use self::runtime::Runtime;
pub(crate) use self::service::Service;
",
    );
    write_rust_file(&src_root, "feature/parser.rs", "pub(crate) struct Parser;");
    write_rust_file(
        &src_root,
        "feature/runtime.rs",
        "pub(crate) struct Runtime;",
    );
    write_rust_file(
        &src_root,
        "feature/service.rs",
        "pub(crate) struct Service;",
    );

    let findings = evaluate_fixture("demo", &temp_dir);
    assert!(
        findings.iter().all(|finding| finding.rule_id != "MOD-R010"),
        "expected no MOD-R010 finding, got {findings:#?}"
    );
}

#[test]
fn modularity_pack_flags_root_seam_without_navigation_hint() {
    let temp_dir = must_ok(TempDir::new(), "should create temp dir");
    let src_root = crate_src_root(&temp_dir, "demo");
    write_rust_file(
        &src_root,
        "feature.rs",
        r"
mod parser;
mod runtime;
mod service;
",
    );
    write_rust_file(&src_root, "feature/parser.rs", "pub(crate) struct Parser;");
    write_rust_file(
        &src_root,
        "feature/runtime.rs",
        "pub(crate) struct Runtime;",
    );
    write_rust_file(
        &src_root,
        "feature/service.rs",
        "pub(crate) struct Service;",
    );

    let findings = evaluate_fixture("demo", &temp_dir);
    let finding = findings
        .iter()
        .find(|finding| finding.rule_id == "MOD-R011")
        .unwrap_or_else(|| panic!("expected MOD-R011 finding, got {findings:#?}"));
    assert!(
        finding.title.contains("navigation hint"),
        "expected root-hint title, got {finding:#?}"
    );
}

#[test]
fn modularity_pack_accepts_root_seam_with_doc_hint() {
    let temp_dir = must_ok(TempDir::new(), "should create temp dir");
    let src_root = crate_src_root(&temp_dir, "demo");
    write_rust_file(
        &src_root,
        "feature.rs",
        r"
//! Parser + runtime seam for the feature.

mod parser;
mod runtime;
mod service;
",
    );
    write_rust_file(&src_root, "feature/parser.rs", "pub(crate) struct Parser;");
    write_rust_file(
        &src_root,
        "feature/runtime.rs",
        "pub(crate) struct Runtime;",
    );
    write_rust_file(
        &src_root,
        "feature/service.rs",
        "pub(crate) struct Service;",
    );

    let findings = evaluate_fixture("demo", &temp_dir);
    assert!(
        findings.iter().all(|finding| finding.rule_id != "MOD-R011"),
        "expected no MOD-R011 finding, got {findings:#?}"
    );
    assert!(
        findings.iter().all(|finding| finding.rule_id != "MOD-R014"),
        "expected no MOD-R014 finding, got {findings:#?}"
    );
}

#[test]
fn modularity_pack_flags_generic_doc_only_root_hint() {
    let temp_dir = must_ok(TempDir::new(), "should create temp dir");
    let src_root = crate_src_root(&temp_dir, "demo");
    write_rust_file(
        &src_root,
        "feature.rs",
        r"
//! Feature seam for the demo.

mod parser;
mod runtime;
mod service;
",
    );
    write_rust_file(&src_root, "feature/parser.rs", "pub(crate) struct Parser;");
    write_rust_file(
        &src_root,
        "feature/runtime.rs",
        "pub(crate) struct Runtime;",
    );
    write_rust_file(
        &src_root,
        "feature/service.rs",
        "pub(crate) struct Service;",
    );

    let findings = evaluate_fixture("demo", &temp_dir);
    let finding = findings
        .iter()
        .find(|finding| finding.rule_id == "MOD-R014")
        .unwrap_or_else(|| panic!("expected MOD-R014 finding, got {findings:#?}"));
    assert!(
        finding.title.contains("name a child module"),
        "expected root-doc-hint title, got {finding:#?}"
    );
}

#[test]
fn modularity_pack_accepts_doc_only_root_hint_that_names_child_module() {
    let temp_dir = must_ok(TempDir::new(), "should create temp dir");
    let src_root = crate_src_root(&temp_dir, "demo");
    write_rust_file(
        &src_root,
        "feature.rs",
        r"
//! Start in `service`, then descend into `parser` for syntax work.

mod parser;
mod runtime;
mod service;
",
    );
    write_rust_file(&src_root, "feature/parser.rs", "pub(crate) struct Parser;");
    write_rust_file(
        &src_root,
        "feature/runtime.rs",
        "pub(crate) struct Runtime;",
    );
    write_rust_file(
        &src_root,
        "feature/service.rs",
        "pub(crate) struct Service;",
    );

    let findings = evaluate_fixture("demo", &temp_dir);
    assert!(
        findings.iter().all(|finding| finding.rule_id != "MOD-R014"),
        "expected no MOD-R014 finding, got {findings:#?}"
    );
}

#[test]
fn modularity_pack_flags_unfocused_root_entry_surface() {
    let temp_dir = must_ok(TempDir::new(), "should create temp dir");
    let src_root = crate_src_root(&temp_dir, "demo");
    write_rust_file(
        &src_root,
        "feature.rs",
        r"
mod parser;
mod runtime;
mod service;

pub(crate) use self::parser::Parser;
pub(crate) use self::runtime::Runtime;
pub(crate) use self::service::Service;
",
    );
    write_rust_file(&src_root, "feature/parser.rs", "pub(crate) struct Parser;");
    write_rust_file(
        &src_root,
        "feature/runtime.rs",
        "pub(crate) struct Runtime;",
    );
    write_rust_file(
        &src_root,
        "feature/service.rs",
        "pub(crate) struct Service;",
    );

    let findings = evaluate_fixture("demo", &temp_dir);
    let finding = findings
        .iter()
        .find(|finding| finding.rule_id == "MOD-R015")
        .unwrap_or_else(|| panic!("expected MOD-R015 finding, got {findings:#?}"));
    assert!(
        finding.title.contains("primary entry owner"),
        "expected root-entry-focus title, got {finding:#?}"
    );
}

#[test]
fn modularity_pack_accepts_root_entry_surface_with_named_primary_owner() {
    let temp_dir = must_ok(TempDir::new(), "should create temp dir");
    let src_root = crate_src_root(&temp_dir, "demo");
    write_rust_file(
        &src_root,
        "feature.rs",
        r"
//! Start in `service`; `parser` and `runtime` support the seam.

mod parser;
mod runtime;
mod service;

pub(crate) use self::parser::Parser;
pub(crate) use self::runtime::Runtime;
pub(crate) use self::service::Service;
",
    );
    write_rust_file(&src_root, "feature/parser.rs", "pub(crate) struct Parser;");
    write_rust_file(
        &src_root,
        "feature/runtime.rs",
        "pub(crate) struct Runtime;",
    );
    write_rust_file(
        &src_root,
        "feature/service.rs",
        "pub(crate) struct Service;",
    );

    let findings = evaluate_fixture("demo", &temp_dir);
    assert!(
        findings.iter().all(|finding| finding.rule_id != "MOD-R015"),
        "expected no MOD-R015 finding, got {findings:#?}"
    );
}

#[test]
fn modularity_pack_flags_root_doc_owner_not_present_in_visible_entries() {
    let temp_dir = must_ok(TempDir::new(), "should create temp dir");
    let src_root = crate_src_root(&temp_dir, "demo");
    write_rust_file(
        &src_root,
        "feature.rs",
        r"
//! Start in `service`.

mod parser;
mod runtime;
mod service;

pub(crate) use self::parser::Parser;
pub(crate) use self::runtime::Runtime;
",
    );
    write_rust_file(&src_root, "feature/parser.rs", "pub(crate) struct Parser;");
    write_rust_file(
        &src_root,
        "feature/runtime.rs",
        "pub(crate) struct Runtime;",
    );
    write_rust_file(
        &src_root,
        "feature/service.rs",
        "pub(crate) struct Service;",
    );

    let findings = evaluate_fixture("demo", &temp_dir);
    let finding = findings
        .iter()
        .find(|finding| finding.rule_id == "MOD-R016")
        .unwrap_or_else(|| panic!("expected MOD-R016 finding, got {findings:#?}"));
    assert!(
        finding.title.contains("align with visible entry surface"),
        "expected root-doc-owner-alignment title, got {finding:#?}"
    );
}

#[test]
fn modularity_pack_accepts_root_doc_owner_present_in_visible_entries() {
    let temp_dir = must_ok(TempDir::new(), "should create temp dir");
    let src_root = crate_src_root(&temp_dir, "demo");
    write_rust_file(
        &src_root,
        "feature.rs",
        r"
//! Start in `service`.

mod parser;
mod runtime;
mod service;

pub(crate) use self::parser::Parser;
pub(crate) use self::service::Service;
",
    );
    write_rust_file(&src_root, "feature/parser.rs", "pub(crate) struct Parser;");
    write_rust_file(
        &src_root,
        "feature/runtime.rs",
        "pub(crate) struct Runtime;",
    );
    write_rust_file(
        &src_root,
        "feature/service.rs",
        "pub(crate) struct Service;",
    );

    let findings = evaluate_fixture("demo", &temp_dir);
    assert!(
        findings.iter().all(|finding| finding.rule_id != "MOD-R016"),
        "expected no MOD-R016 finding, got {findings:#?}"
    );
}

#[test]
fn modularity_pack_flags_root_owner_convergence_drift() {
    let temp_dir = must_ok(TempDir::new(), "should create temp dir");
    let src_root = crate_src_root(&temp_dir, "demo");
    write_rust_file(
        &src_root,
        "feature.rs",
        r"
//! Start in `service`.

mod parser;
mod runtime;
mod service;

pub(crate) use self::parser::{ParsePlan, Parser};
pub(crate) use self::service::Service;
",
    );
    write_rust_file(
        &src_root,
        "feature/parser.rs",
        r"
pub(crate) struct Parser;
pub(crate) struct ParsePlan;
",
    );
    write_rust_file(
        &src_root,
        "feature/runtime.rs",
        "pub(crate) struct Runtime;",
    );
    write_rust_file(
        &src_root,
        "feature/service.rs",
        "pub(crate) struct Service;",
    );

    let findings = evaluate_fixture("demo", &temp_dir);
    let finding = findings
        .iter()
        .find(|finding| finding.rule_id == "MOD-R017")
        .unwrap_or_else(|| panic!("expected MOD-R017 finding, got {findings:#?}"));
    assert!(
        finding.title.contains("converge on one owner"),
        "expected root-owner-convergence title, got {finding:#?}"
    );
}

#[test]
fn modularity_pack_accepts_converged_root_owner_surface() {
    let temp_dir = must_ok(TempDir::new(), "should create temp dir");
    let src_root = crate_src_root(&temp_dir, "demo");
    write_rust_file(
        &src_root,
        "feature.rs",
        r"
//! Start in `service`.

mod parser;
mod runtime;
mod service;

pub(crate) use self::parser::Parser;
pub(crate) use self::service::{Service, ServicePlan};
",
    );
    write_rust_file(&src_root, "feature/parser.rs", "pub(crate) struct Parser;");
    write_rust_file(
        &src_root,
        "feature/runtime.rs",
        "pub(crate) struct Runtime;",
    );
    write_rust_file(
        &src_root,
        "feature/service.rs",
        r"
pub(crate) struct Service;
pub(crate) struct ServicePlan;
",
    );

    let findings = evaluate_fixture("demo", &temp_dir);
    assert!(
        findings.iter().all(|finding| finding.rule_id != "MOD-R017"),
        "expected no MOD-R017 finding, got {findings:#?}"
    );
}

#[test]
fn modularity_pack_flags_plain_pub_entry_in_internal_root_seam() {
    let temp_dir = must_ok(TempDir::new(), "should create temp dir");
    let src_root = crate_src_root(&temp_dir, "demo");
    write_rust_file(
        &src_root,
        "lib.rs",
        r"
mod feature;
",
    );
    write_rust_file(
        &src_root,
        "feature.rs",
        r"
mod parser;
mod runtime;
mod service;

pub use self::service::Service;
",
    );
    write_rust_file(&src_root, "feature/parser.rs", "pub(crate) struct Parser;");
    write_rust_file(
        &src_root,
        "feature/runtime.rs",
        "pub(crate) struct Runtime;",
    );
    write_rust_file(
        &src_root,
        "feature/service.rs",
        "pub(crate) struct Service;",
    );

    let findings = evaluate_fixture("demo", &temp_dir);
    let finding = findings
        .iter()
        .find(|finding| finding.rule_id == "MOD-R018")
        .unwrap_or_else(|| panic!("expected MOD-R018 finding, got {findings:#?}"));
    assert!(
        finding.title.contains("restricted entry visibility"),
        "expected root-entry-visibility title, got {finding:#?}"
    );
}

#[test]
fn modularity_pack_accepts_restricted_entry_in_internal_root_seam() {
    let temp_dir = must_ok(TempDir::new(), "should create temp dir");
    let src_root = crate_src_root(&temp_dir, "demo");
    write_rust_file(
        &src_root,
        "lib.rs",
        r"
mod feature;
",
    );
    write_rust_file(
        &src_root,
        "feature.rs",
        r"
mod parser;
mod runtime;
mod service;

pub(crate) use self::service::Service;
",
    );
    write_rust_file(&src_root, "feature/parser.rs", "pub(crate) struct Parser;");
    write_rust_file(
        &src_root,
        "feature/runtime.rs",
        "pub(crate) struct Runtime;",
    );
    write_rust_file(
        &src_root,
        "feature/service.rs",
        "pub(crate) struct Service;",
    );

    let findings = evaluate_fixture("demo", &temp_dir);
    assert!(
        findings.iter().all(|finding| finding.rule_id != "MOD-R018"),
        "expected no MOD-R018 finding, got {findings:#?}"
    );
}

#[test]
fn modularity_pack_accepts_plain_pub_entry_when_parent_module_is_public() {
    let temp_dir = must_ok(TempDir::new(), "should create temp dir");
    let src_root = crate_src_root(&temp_dir, "demo");
    write_rust_file(
        &src_root,
        "lib.rs",
        r"
pub mod feature;
",
    );
    write_rust_file(
        &src_root,
        "feature.rs",
        r"
mod parser;
mod runtime;
mod service;

pub use self::service::Service;
",
    );
    write_rust_file(&src_root, "feature/parser.rs", "pub(crate) struct Parser;");
    write_rust_file(
        &src_root,
        "feature/runtime.rs",
        "pub(crate) struct Runtime;",
    );
    write_rust_file(
        &src_root,
        "feature/service.rs",
        "pub(crate) struct Service;",
    );

    let findings = evaluate_fixture("demo", &temp_dir);
    assert!(
        findings.iter().all(|finding| finding.rule_id != "MOD-R018"),
        "expected no MOD-R018 finding, got {findings:#?}"
    );
}

#[test]
fn modularity_pack_flags_internal_root_seam_with_multiple_visible_owners() {
    let temp_dir = must_ok(TempDir::new(), "should create temp dir");
    let src_root = crate_src_root(&temp_dir, "demo");
    write_rust_file(
        &src_root,
        "lib.rs",
        r"
mod feature;
",
    );
    write_rust_file(
        &src_root,
        "feature.rs",
        r"
//! Start in `service`.

mod parser;
mod runtime;
mod service;

pub(crate) use self::parser::Parser;
pub(crate) use self::service::Service;
",
    );
    write_rust_file(&src_root, "feature/parser.rs", "pub(crate) struct Parser;");
    write_rust_file(
        &src_root,
        "feature/runtime.rs",
        "pub(crate) struct Runtime;",
    );
    write_rust_file(
        &src_root,
        "feature/service.rs",
        "pub(crate) struct Service;",
    );

    let findings = evaluate_fixture("demo", &temp_dir);
    let finding = findings
        .iter()
        .find(|finding| finding.rule_id == "MOD-R019")
        .unwrap_or_else(|| panic!("expected MOD-R019 finding, got {findings:#?}"));
    assert!(
        finding.title.contains("canonical visible owner"),
        "expected root-entry-curation title, got {finding:#?}"
    );
}

#[test]
fn modularity_pack_accepts_internal_root_seam_with_one_visible_owner() {
    let temp_dir = must_ok(TempDir::new(), "should create temp dir");
    let src_root = crate_src_root(&temp_dir, "demo");
    write_rust_file(
        &src_root,
        "lib.rs",
        r"
mod feature;
",
    );
    write_rust_file(
        &src_root,
        "feature.rs",
        r"
//! Start in `service`; parser and runtime stay leaf-owned.

mod parser;
mod runtime;
mod service;

pub(crate) use self::service::Service;
",
    );
    write_rust_file(&src_root, "feature/parser.rs", "pub(crate) struct Parser;");
    write_rust_file(
        &src_root,
        "feature/runtime.rs",
        "pub(crate) struct Runtime;",
    );
    write_rust_file(
        &src_root,
        "feature/service.rs",
        "pub(crate) struct Service;",
    );

    let findings = evaluate_fixture("demo", &temp_dir);
    assert!(
        findings.iter().all(|finding| finding.rule_id != "MOD-R019"),
        "expected no MOD-R019 finding, got {findings:#?}"
    );
}

#[test]
fn modularity_pack_accepts_public_root_seam_with_multiple_visible_owners() {
    let temp_dir = must_ok(TempDir::new(), "should create temp dir");
    let src_root = crate_src_root(&temp_dir, "demo");
    write_rust_file(
        &src_root,
        "lib.rs",
        r"
pub mod feature;
",
    );
    write_rust_file(
        &src_root,
        "feature.rs",
        r"
mod parser;
mod runtime;
mod service;

pub use self::parser::Parser;
pub use self::service::Service;
",
    );
    write_rust_file(&src_root, "feature/parser.rs", "pub(crate) struct Parser;");
    write_rust_file(
        &src_root,
        "feature/runtime.rs",
        "pub(crate) struct Runtime;",
    );
    write_rust_file(
        &src_root,
        "feature/service.rs",
        "pub(crate) struct Service;",
    );

    let findings = evaluate_fixture("demo", &temp_dir);
    assert!(
        findings.iter().all(|finding| finding.rule_id != "MOD-R019"),
        "expected no MOD-R019 finding, got {findings:#?}"
    );
}

#[test]
fn modularity_pack_flags_inventory_style_root_doc_for_internal_canonical_owner() {
    let temp_dir = must_ok(TempDir::new(), "should create temp dir");
    let src_root = crate_src_root(&temp_dir, "demo");
    write_rust_file(
        &src_root,
        "lib.rs",
        r"
mod feature;
",
    );
    write_rust_file(
        &src_root,
        "feature.rs",
        r"
//! Start in `service`; parser handles syntax; runtime executes requests.

mod parser;
mod runtime;
mod service;

pub(crate) use self::service::Service;
",
    );
    write_rust_file(&src_root, "feature/parser.rs", "pub(crate) struct Parser;");
    write_rust_file(
        &src_root,
        "feature/runtime.rs",
        "pub(crate) struct Runtime;",
    );
    write_rust_file(
        &src_root,
        "feature/service.rs",
        "pub(crate) struct Service;",
    );

    let findings = evaluate_fixture("demo", &temp_dir);
    let finding = findings
        .iter()
        .find(|finding| finding.rule_id == "MOD-R020")
        .unwrap_or_else(|| panic!("expected MOD-R020 finding, got {findings:#?}"));
    assert!(
        finding.title.contains("canonical owner"),
        "expected root-doc-curation title, got {finding:#?}"
    );
}

#[test]
fn modularity_pack_accepts_focused_root_doc_for_internal_canonical_owner() {
    let temp_dir = must_ok(TempDir::new(), "should create temp dir");
    let src_root = crate_src_root(&temp_dir, "demo");
    write_rust_file(
        &src_root,
        "lib.rs",
        r"
mod feature;
",
    );
    write_rust_file(
        &src_root,
        "feature.rs",
        r"
//! Start in `service`; parser stays leaf-owned.

mod parser;
mod runtime;
mod service;

pub(crate) use self::service::Service;
",
    );
    write_rust_file(&src_root, "feature/parser.rs", "pub(crate) struct Parser;");
    write_rust_file(
        &src_root,
        "feature/runtime.rs",
        "pub(crate) struct Runtime;",
    );
    write_rust_file(
        &src_root,
        "feature/service.rs",
        "pub(crate) struct Service;",
    );

    let findings = evaluate_fixture("demo", &temp_dir);
    assert!(
        findings.iter().all(|finding| finding.rule_id != "MOD-R020"),
        "expected no MOD-R020 finding, got {findings:#?}"
    );
}

#[test]
fn modularity_pack_accepts_inventory_style_root_doc_for_public_parent() {
    let temp_dir = must_ok(TempDir::new(), "should create temp dir");
    let src_root = crate_src_root(&temp_dir, "demo");
    write_rust_file(
        &src_root,
        "lib.rs",
        r"
pub mod feature;
",
    );
    write_rust_file(
        &src_root,
        "feature.rs",
        r"
//! Start in `service`; parser handles syntax; runtime executes requests.

mod parser;
mod runtime;
mod service;

pub use self::service::Service;
",
    );
    write_rust_file(&src_root, "feature/parser.rs", "pub(crate) struct Parser;");
    write_rust_file(
        &src_root,
        "feature/runtime.rs",
        "pub(crate) struct Runtime;",
    );
    write_rust_file(
        &src_root,
        "feature/service.rs",
        "pub(crate) struct Service;",
    );

    let findings = evaluate_fixture("demo", &temp_dir);
    assert!(
        findings.iter().all(|finding| finding.rule_id != "MOD-R020"),
        "expected no MOD-R020 finding, got {findings:#?}"
    );
}

#[test]
fn modularity_pack_flags_root_entry_from_internal_module() {
    let temp_dir = must_ok(TempDir::new(), "should create temp dir");
    let src_root = crate_src_root(&temp_dir, "demo");
    write_rust_file(
        &src_root,
        "feature.rs",
        r"
mod internal;
mod parser;
mod service;

pub(crate) use self::internal::FeatureState;
pub(crate) use self::service::Service;
",
    );
    write_rust_file(
        &src_root,
        "feature/internal.rs",
        "pub(crate) struct FeatureState;",
    );
    write_rust_file(&src_root, "feature/parser.rs", "pub(crate) struct Parser;");
    write_rust_file(
        &src_root,
        "feature/service.rs",
        "pub(crate) struct Service;",
    );

    let findings = evaluate_fixture("demo", &temp_dir);
    let finding = findings
        .iter()
        .find(|finding| finding.rule_id == "MOD-R012")
        .unwrap_or_else(|| panic!("expected MOD-R012 finding, got {findings:#?}"));
    assert!(
        finding.title.contains("helper modules"),
        "expected root-owner title, got {finding:#?}"
    );
}

#[test]
fn modularity_pack_accepts_root_entry_from_canonical_module() {
    let temp_dir = must_ok(TempDir::new(), "should create temp dir");
    let src_root = crate_src_root(&temp_dir, "demo");
    write_rust_file(
        &src_root,
        "feature.rs",
        r"
mod parser;
mod runtime;
mod service;

pub(crate) use self::service::Service;
pub(crate) use self::runtime::Runtime;
",
    );
    write_rust_file(&src_root, "feature/parser.rs", "pub(crate) struct Parser;");
    write_rust_file(
        &src_root,
        "feature/runtime.rs",
        "pub(crate) struct Runtime;",
    );
    write_rust_file(
        &src_root,
        "feature/service.rs",
        "pub(crate) struct Service;",
    );

    let findings = evaluate_fixture("demo", &temp_dir);
    assert!(
        findings.iter().all(|finding| finding.rule_id != "MOD-R012"),
        "expected no MOD-R012 finding, got {findings:#?}"
    );
}

#[test]
fn modularity_pack_flags_visible_child_module_in_root_facade() {
    let temp_dir = must_ok(TempDir::new(), "should create temp dir");
    let src_root = crate_src_root(&temp_dir, "demo");
    write_rust_file(
        &src_root,
        "feature.rs",
        r"
mod parser;
pub(crate) mod service;
mod runtime;

pub(crate) use self::service::Service;
",
    );
    write_rust_file(&src_root, "feature/parser.rs", "pub(crate) struct Parser;");
    write_rust_file(
        &src_root,
        "feature/service.rs",
        "pub(crate) struct Service;",
    );
    write_rust_file(
        &src_root,
        "feature/runtime.rs",
        "pub(crate) struct Runtime;",
    );

    let findings = evaluate_fixture("demo", &temp_dir);
    let finding = findings
        .iter()
        .find(|finding| finding.rule_id == "MOD-R013")
        .unwrap_or_else(|| panic!("expected MOD-R013 finding, got {findings:#?}"));
    assert!(
        finding.title.contains("child modules private"),
        "expected root-child-visibility title, got {finding:#?}"
    );
}

#[test]
fn modularity_pack_accepts_private_child_modules_in_root_facade() {
    let temp_dir = must_ok(TempDir::new(), "should create temp dir");
    let src_root = crate_src_root(&temp_dir, "demo");
    write_rust_file(
        &src_root,
        "feature.rs",
        r"
mod parser;
mod runtime;
mod service;

pub(crate) use self::service::Service;
",
    );
    write_rust_file(&src_root, "feature/parser.rs", "pub(crate) struct Parser;");
    write_rust_file(
        &src_root,
        "feature/runtime.rs",
        "pub(crate) struct Runtime;",
    );
    write_rust_file(
        &src_root,
        "feature/service.rs",
        "pub(crate) struct Service;",
    );

    let findings = evaluate_fixture("demo", &temp_dir);
    assert!(
        findings.iter().all(|finding| finding.rule_id != "MOD-R013"),
        "expected no MOD-R013 finding, got {findings:#?}"
    );
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

#[test]
fn modularity_pack_accepts_multiline_interface_exports_in_mod_rs() {
    let temp_dir = must_ok(TempDir::new(), "should create temp dir");
    let src_root = crate_src_root(&temp_dir, "demo");
    write_rust_file(
        &src_root,
        "feature/mod.rs",
        r"
#![allow(dead_code)]
mod parser;
mod scanner;

pub use self::{
    parser::Parser,
    scanner::Scanner,
};
pub(super) use self::parser::Parser as InternalParser;
",
    );
    write_rust_file(&src_root, "feature/parser.rs", "pub(crate) struct Parser;");
    write_rust_file(
        &src_root,
        "feature/scanner.rs",
        "pub(crate) struct Scanner;",
    );

    let findings = evaluate_fixture("demo", &temp_dir);
    assert!(
        findings.iter().all(|finding| finding.rule_id != "MOD-R001"),
        "expected no MOD-R001 finding, got {findings:#?}"
    );
}

#[test]
fn modularity_pack_accepts_explicit_restricted_reexports_in_mod_rs() {
    let temp_dir = must_ok(TempDir::new(), "should create temp dir");
    let src_root = crate_src_root(&temp_dir, "demo");
    write_rust_file(
        &src_root,
        "feature/mod.rs",
        r#"
#[cfg(test)]
#[path = "../../tests/unit/feature/mod.rs"]
mod tests;
mod parser;
mod scanner;

pub(crate) use self::parser::Parser;
pub(super) use self::scanner::Scanner as InternalScanner;
"#,
    );
    write_rust_file(&src_root, "feature/parser.rs", "pub(crate) struct Parser;");
    write_rust_file(
        &src_root,
        "feature/scanner.rs",
        "pub(crate) struct Scanner;",
    );

    let findings = evaluate_fixture("demo", &temp_dir);
    assert!(
        findings.iter().all(|finding| finding.rule_id != "MOD-R001"),
        "expected no MOD-R001 finding, got {findings:#?}"
    );
}
