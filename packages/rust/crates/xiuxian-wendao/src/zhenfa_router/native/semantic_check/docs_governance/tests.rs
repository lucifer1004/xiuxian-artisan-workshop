//! Tests for docs governance module.

use std::collections::HashMap;
use std::fs;
use std::path::Path;

use tempfile::TempDir;
use xiuxian_zhenfa::ZhenfaContext;

use super::*;
use crate::link_graph::LinkGraphIndex;
use crate::zhenfa_router::native::audit::fix::AtomicFixBatch;
use crate::zhenfa_router::native::audit::generate_surgical_fixes;
use crate::zhenfa_router::native::semantic_check::docs_governance::collection::collect_stale_index_footer_standards;
use crate::zhenfa_router::native::semantic_check::docs_governance::parsing::derive_opaque_doc_id;
use crate::zhenfa_router::native::semantic_check::{
    CheckType, WendaoSemanticCheckArgs, run_audit_core,
};

trait PanicExt<T> {
    fn or_panic(self, context: &str) -> T;
}

impl<T, E> PanicExt<T> for Result<T, E>
where
    E: std::fmt::Display,
{
    fn or_panic(self, context: &str) -> T {
        self.unwrap_or_else(|error| panic!("{context}: {error}"))
    }
}

impl<T> PanicExt<T> for Option<T> {
    fn or_panic(self, context: &str) -> T {
        self.unwrap_or_else(|| panic!("{context}"))
    }
}

#[test]
fn detects_non_opaque_doc_identity_for_package_local_docs() {
    let content = "# Title\n\n:PROPERTIES:\n:ID: readable-id\n:TYPE: CORE\n:END:\n";
    let doc_path = "packages/rust/crates/demo/docs/01_core/101_test.md";
    let issues = collect_doc_governance_issues(doc_path, content);
    let expected = derive_opaque_doc_id(doc_path);

    assert_eq!(issues.len(), 1);
    assert_eq!(issues[0].issue_type, DOC_IDENTITY_PROTOCOL_ISSUE_TYPE);
    assert_eq!(issues[0].suggestion.as_deref(), Some(expected.as_str()));
    assert_eq!(issues[0].location.as_ref().map(|loc| loc.line), Some(4));
}

#[test]
fn detects_missing_doc_identity_inside_top_properties_drawer() {
    let content = "# Title\n\n:PROPERTIES:\n:TYPE: CORE\n:END:\n";
    let doc_path = "packages/rust/crates/demo/docs/01_core/101_test.md";
    let issues = collect_doc_governance_issues(doc_path, content);
    let expected = format!(":ID: {}\n", derive_opaque_doc_id(doc_path));

    assert_eq!(issues.len(), 1);
    assert_eq!(issues[0].issue_type, DOC_IDENTITY_PROTOCOL_ISSUE_TYPE);
    assert_eq!(issues[0].suggestion.as_deref(), Some(expected.as_str()));
    assert_eq!(issues[0].location.as_ref().map(|loc| loc.line), Some(4));
    assert_eq!(
        issues[0].location.as_ref().and_then(|loc| loc.byte_range),
        Some((22, 22))
    );
}

#[test]
fn ignores_docs_outside_package_local_crate_docs() {
    let content = "# Title\n\n:PROPERTIES:\n:ID: readable-id\n:TYPE: CORE\n:END:\n";
    let issues = collect_doc_governance_issues("docs/notes.md", content);
    assert!(issues.is_empty());
}

#[test]
fn surgical_fixes_repair_non_opaque_doc_identity() {
    let doc_key = "packages/rust/crates/demo/docs/01_core/101_external_modelica_plugin_boundary.md";
    let original = "# Demo\n\n:PROPERTIES:\n:ID: readable-id\n:TYPE: CORE\n:END:\n\nBody.\n";
    let issues = collect_doc_governance_issues(doc_key, original);
    assert_eq!(issues.len(), 1);

    let file_contents = HashMap::from([(doc_key.to_string(), original.to_string())]);
    let fixes = generate_surgical_fixes(&issues, &file_contents);
    assert_eq!(fixes.len(), 1);

    let mut content = original.to_string();
    let result = fixes[0].apply_surgical(&mut content);
    assert!(matches!(
        result,
        crate::zhenfa_router::native::audit::FixResult::Success
    ));
    assert!(content.contains(&format!(":ID: {}", derive_opaque_doc_id(doc_key))));
}

#[test]
fn detects_missing_package_docs_index_for_workspace_crate_docs() {
    let temp = TempDir::new().or_panic("tempdir");
    let crate_dir = temp.path().join("packages/rust/crates/demo");
    fs::create_dir_all(&crate_dir).or_panic("create crate dir");
    fs::write(
        crate_dir.join("Cargo.toml"),
        "[package]\nname = \"demo\"\nversion = \"0.1.0\"\n",
    )
    .or_panic("write cargo");
    let docs_dir = temp.path().join("packages/rust/crates/demo/docs/01_core");
    fs::create_dir_all(&docs_dir).or_panic("create docs dir");
    let doc_path = docs_dir.join("101_intro.md");
    let doc_path_str = doc_path.to_string_lossy().to_string();
    let content = format!(
        "# Intro\n\n:PROPERTIES:\n:ID: {}\n:END:\n\nIntro.\n",
        derive_opaque_doc_id(&doc_path_str)
    );
    fs::write(&doc_path, content).or_panic("write doc");

    let issues = collect_workspace_doc_governance_issues(temp.path(), None);
    assert_eq!(issues.len(), 1);
    assert_eq!(issues[0].issue_type, MISSING_PACKAGE_DOCS_INDEX_ISSUE_TYPE);
    assert!(
        issues[0]
            .doc
            .ends_with("packages/rust/crates/demo/docs/index.md")
    );
    let suggestion = issues[0].suggestion.as_ref().or_panic("suggestion");
    assert!(suggestion.contains("# demo: Map of Content"));
    assert!(suggestion.contains("[[01_core/101_intro]]"));
}

#[test]
fn detects_missing_package_docs_tree_for_workspace_crate() {
    let temp = TempDir::new().or_panic("tempdir");
    let crate_dir = temp.path().join("packages/rust/crates/demo");
    fs::create_dir_all(&crate_dir).or_panic("create crate dir");
    fs::write(
        crate_dir.join("Cargo.toml"),
        "[package]\nname = \"demo\"\nversion = \"0.1.0\"\n",
    )
    .or_panic("write cargo");

    let issues = collect_workspace_doc_governance_issues(temp.path(), None);
    assert_eq!(issues.len(), 1);
    assert_eq!(issues[0].issue_type, MISSING_PACKAGE_DOCS_TREE_ISSUE_TYPE);
    assert_eq!(issues[0].severity, "warning");
    assert!(
        issues[0]
            .doc
            .ends_with("packages/rust/crates/demo/docs/index.md")
    );
    let suggestion = issues[0].suggestion.as_ref().or_panic("suggestion");
    assert!(suggestion.contains("# demo: Map of Content"));
    assert!(suggestion.contains("Standardized documentation index"));
}

#[test]
fn detects_doc_identity_for_workspace_package_docs_tree_files() {
    let temp = TempDir::new().or_panic("tempdir");
    let crate_dir = temp.path().join("packages/rust/crates/demo");
    let core_dir = crate_dir.join("docs/01_core");
    fs::create_dir_all(&core_dir).or_panic("create core docs dir");
    fs::write(
        crate_dir.join("Cargo.toml"),
        "[package]\nname = \"demo\"\nversion = \"0.1.0\"\n",
    )
    .or_panic("write cargo");

    let index_path = crate_dir.join("docs/index.md");
    let index_path_str = index_path.to_string_lossy().to_string();
    fs::write(
        &index_path,
        format!(
            "# Demo\n\n:PROPERTIES:\n:ID: {}\n:TYPE: INDEX\n:END:\n",
            derive_opaque_doc_id(&index_path_str)
        ),
    )
    .or_panic("write index");

    let intro_path = core_dir.join("101_intro.md");
    let intro_path_str = intro_path.to_string_lossy().to_string();
    fs::write(
        &intro_path,
        "# Intro\n\n:PROPERTIES:\n:ID: readable-intro\n:TYPE: CORE\n:END:\n",
    )
    .or_panic("write intro");

    let issues = collect_workspace_doc_governance_issues(temp.path(), None);
    let issue = issues
        .iter()
        .find(|issue| {
            issue.issue_type == DOC_IDENTITY_PROTOCOL_ISSUE_TYPE && issue.doc == intro_path_str
        })
        .or_panic("workspace doc identity issue");

    assert_eq!(issue.severity, "error");
    assert_eq!(
        issue.suggestion.as_deref(),
        Some(derive_opaque_doc_id(&intro_path_str).as_str())
    );
}

#[test]
fn workspace_doc_identity_scan_respects_explicit_doc_scope() {
    let temp = TempDir::new().or_panic("tempdir");
    let crate_dir = temp.path().join("packages/rust/crates/demo");
    let core_dir = crate_dir.join("docs/01_core");
    fs::create_dir_all(&core_dir).or_panic("create core docs dir");
    fs::write(
        crate_dir.join("Cargo.toml"),
        "[package]\nname = \"demo\"\nversion = \"0.1.0\"\n",
    )
    .or_panic("write cargo");

    let index_path = crate_dir.join("docs/index.md");
    let index_path_str = index_path.to_string_lossy().to_string();
    fs::write(
        &index_path,
        format!(
            "# Demo\n\n:PROPERTIES:\n:ID: {}\n:TYPE: INDEX\n:END:\n",
            derive_opaque_doc_id(&index_path_str)
        ),
    )
    .or_panic("write index");

    let intro_path = core_dir.join("101_intro.md");
    let intro_path_str = intro_path.to_string_lossy().to_string();
    fs::write(
        &intro_path,
        "# Intro\n\n:PROPERTIES:\n:ID: readable-intro\n:TYPE: CORE\n:END:\n",
    )
    .or_panic("write intro");

    let contracts_path = core_dir.join("102_contracts.md");
    fs::write(
        &contracts_path,
        "# Contracts\n\n:PROPERTIES:\n:ID: readable-contracts\n:TYPE: CORE\n:END:\n",
    )
    .or_panic("write contracts");

    let issues = collect_workspace_doc_governance_issues(temp.path(), Some(&intro_path_str));
    let identity_issues = issues
        .iter()
        .filter(|issue| issue.issue_type == DOC_IDENTITY_PROTOCOL_ISSUE_TYPE)
        .collect::<Vec<_>>();

    assert_eq!(identity_issues.len(), 1);
    assert_eq!(
        Path::new(&identity_issues[0].doc)
            .canonicalize()
            .or_panic("canonical issue path"),
        intro_path.canonicalize().or_panic("canonical intro path")
    );
}

#[test]
fn workspace_scope_does_not_match_prefix_sibling_crates() {
    let temp = TempDir::new().or_panic("tempdir");
    let wendao_dir = temp.path().join("packages/rust/crates/xiuxian-wendao");
    let modelica_dir = temp
        .path()
        .join("packages/rust/crates/xiuxian-wendao-modelica");

    for crate_dir in [&wendao_dir, &modelica_dir] {
        fs::create_dir_all(crate_dir.join("docs/01_core")).or_panic("create docs dir");
        fs::write(
            crate_dir.join("Cargo.toml"),
            format!(
                "[package]\nname = \"{}\"\nversion = \"0.1.0\"\n",
                crate_dir
                    .file_name()
                    .and_then(|name| name.to_str())
                    .or_panic("crate name")
            ),
        )
        .or_panic("write cargo");
    }

    let wendao_doc = wendao_dir.join("docs/01_core/101_core.md");
    fs::write(
        &wendao_doc,
        "# Wendao\n\n:PROPERTIES:\n:ID: readable-wendao\n:TYPE: CORE\n:END:\n",
    )
    .or_panic("write wendao doc");

    let modelica_doc = modelica_dir.join("docs/01_core/101_core.md");
    fs::write(
        &modelica_doc,
        "# Modelica\n\n:PROPERTIES:\n:ID: readable-modelica\n:TYPE: CORE\n:END:\n",
    )
    .or_panic("write modelica doc");

    let issues = collect_workspace_doc_governance_issues(
        temp.path(),
        Some(&modelica_dir.join("docs").to_string_lossy()),
    );
    let identity_issues = issues
        .iter()
        .filter(|issue| issue.issue_type == DOC_IDENTITY_PROTOCOL_ISSUE_TYPE)
        .collect::<Vec<_>>();

    assert_eq!(identity_issues.len(), 1);
    assert_eq!(
        Path::new(&identity_issues[0].doc)
            .canonicalize()
            .or_panic("canonical issue path"),
        modelica_doc
            .canonicalize()
            .or_panic("canonical modelica doc")
    );
}

#[test]
fn run_audit_core_reports_missing_package_docs_index() {
    let temp = TempDir::new().or_panic("tempdir");
    let crate_dir = temp.path().join("packages/rust/crates/demo");
    fs::create_dir_all(&crate_dir).or_panic("create crate dir");
    fs::write(
        crate_dir.join("Cargo.toml"),
        "[package]\nname = \"demo\"\nversion = \"0.1.0\"\n",
    )
    .or_panic("write cargo");
    let docs_dir = temp.path().join("packages/rust/crates/demo/docs/01_core");
    fs::create_dir_all(&docs_dir).or_panic("create docs dir");
    let doc_path = docs_dir.join("101_intro.md");
    let doc_path_str = doc_path.to_string_lossy().to_string();
    let content = format!(
        "# Intro\n\n:PROPERTIES:\n:ID: {}\n:END:\n\nIntro.\n",
        derive_opaque_doc_id(&doc_path_str)
    );
    fs::write(&doc_path, content).or_panic("write doc");

    let index = LinkGraphIndex::build(temp.path()).or_panic("build index");
    let mut ctx = ZhenfaContext::default();
    ctx.insert_extension(index);

    let args = WendaoSemanticCheckArgs {
        doc: Some(".".to_string()),
        checks: Some(vec![CheckType::DocGovernance]),
        include_warnings: Some(true),
        source_paths: None,
        fuzzy_confidence_threshold: None,
    };
    let (issues, _file_contents) = run_audit_core(&ctx, &args).or_panic("audit");

    assert!(
        issues
            .iter()
            .any(|issue| issue.issue_type == MISSING_PACKAGE_DOCS_INDEX_ISSUE_TYPE)
    );
}

#[test]
fn run_audit_core_reports_missing_package_docs_tree() {
    let temp = TempDir::new().or_panic("tempdir");
    let crate_dir = temp.path().join("packages/rust/crates/demo");
    fs::create_dir_all(&crate_dir).or_panic("create crate dir");
    fs::write(
        crate_dir.join("Cargo.toml"),
        "[package]\nname = \"demo\"\nversion = \"0.1.0\"\n",
    )
    .or_panic("write cargo");

    let index = LinkGraphIndex::build(temp.path()).or_panic("build index");
    let mut ctx = ZhenfaContext::default();
    ctx.insert_extension(index);

    let args = WendaoSemanticCheckArgs {
        doc: Some(".".to_string()),
        checks: Some(vec![CheckType::DocGovernance]),
        include_warnings: Some(true),
        source_paths: None,
        fuzzy_confidence_threshold: None,
    };
    let (issues, _file_contents) = run_audit_core(&ctx, &args).or_panic("audit");

    assert!(
        issues
            .iter()
            .any(|issue| issue.issue_type == MISSING_PACKAGE_DOCS_TREE_ISSUE_TYPE)
    );
}

#[test]
fn detects_missing_standard_section_landings_for_existing_docs_tree() {
    let temp = TempDir::new().or_panic("tempdir");
    let crate_dir = temp.path().join("packages/rust/crates/demo");
    fs::create_dir_all(crate_dir.join("docs")).or_panic("create docs dir");
    fs::write(
        crate_dir.join("Cargo.toml"),
        "[package]\nname = \"demo\"\nversion = \"0.1.0\"\n",
    )
    .or_panic("write cargo");
    let index_path = crate_dir.join("docs/index.md");
    let index_path_str = index_path.to_string_lossy().to_string();
    fs::write(
        &index_path,
        format!(
            "# Demo\n\n:PROPERTIES:\n:ID: {}\n:END:\n",
            derive_opaque_doc_id(&index_path_str)
        ),
    )
    .or_panic("write index");

    let issues = collect_workspace_doc_governance_issues(temp.path(), None);
    let section_issues: Vec<_> = issues
        .iter()
        .filter(|issue| issue.issue_type == MISSING_PACKAGE_DOCS_SECTION_LANDING_ISSUE_TYPE)
        .collect();

    assert_eq!(section_issues.len(), 4);
    assert!(
        section_issues
            .iter()
            .any(|issue| issue.doc.ends_with("01_core/101_demo_core_boundary.md"))
    );
    assert!(section_issues.iter().any(|issue| {
        issue
            .doc
            .ends_with("03_features/201_demo_feature_ledger.md")
    }));
    assert!(section_issues.iter().any(|issue| {
        issue
            .doc
            .ends_with("05_research/301_demo_research_agenda.md")
    }));
    assert!(
        section_issues
            .iter()
            .any(|issue| issue.doc.ends_with("06_roadmap/401_demo_roadmap.md"))
    );
}

#[test]
fn run_audit_core_reports_missing_standard_section_landings() {
    let temp = TempDir::new().or_panic("tempdir");
    let crate_dir = temp.path().join("packages/rust/crates/demo");
    fs::create_dir_all(crate_dir.join("docs")).or_panic("create docs dir");
    fs::write(
        crate_dir.join("Cargo.toml"),
        "[package]\nname = \"demo\"\nversion = \"0.1.0\"\n",
    )
    .or_panic("write cargo");
    let index_path = crate_dir.join("docs/index.md");
    let index_path_str = index_path.to_string_lossy().to_string();
    fs::write(
        &index_path,
        format!(
            "# Demo\n\n:PROPERTIES:\n:ID: {}\n:END:\n",
            derive_opaque_doc_id(&index_path_str)
        ),
    )
    .or_panic("write index");

    let index = LinkGraphIndex::build(temp.path()).or_panic("build index");
    let mut ctx = ZhenfaContext::default();
    ctx.insert_extension(index);

    let args = WendaoSemanticCheckArgs {
        doc: Some(".".to_string()),
        checks: Some(vec![CheckType::DocGovernance]),
        include_warnings: Some(true),
        source_paths: None,
        fuzzy_confidence_threshold: None,
    };
    let (issues, _file_contents) = run_audit_core(&ctx, &args).or_panic("audit");

    assert!(issues.iter().any(|issue| {
        issue.issue_type == MISSING_PACKAGE_DOCS_SECTION_LANDING_ISSUE_TYPE
            && issue
                .doc
                .ends_with("03_features/201_demo_feature_ledger.md")
    }));
}

#[test]
fn detects_missing_standard_index_section_links_for_existing_landing_pages() {
    let temp = TempDir::new().or_panic("tempdir");
    let crate_dir = temp.path().join("packages/rust/crates/demo");
    let core_dir = crate_dir.join("docs/01_core");
    fs::create_dir_all(&core_dir).or_panic("create core docs dir");
    fs::write(
        crate_dir.join("Cargo.toml"),
        "[package]\nname = \"demo\"\nversion = \"0.1.0\"\n",
    )
    .or_panic("write cargo");

    let index_path = crate_dir.join("docs/index.md");
    let index_path_str = index_path.to_string_lossy().to_string();
    fs::write(
        &index_path,
        format!(
            "# Demo\n\n:PROPERTIES:\n:ID: {}\n:END:\n\n## 01_core: Architecture and Foundation\n\n",
            derive_opaque_doc_id(&index_path_str)
        ),
    )
    .or_panic("write index");

    let landing_path = core_dir.join("101_demo_core_boundary.md");
    let landing_path_str = landing_path.to_string_lossy().to_string();
    fs::write(
        &landing_path,
        format!(
            "# Core Boundary\n\n:PROPERTIES:\n:ID: {}\n:END:\n",
            derive_opaque_doc_id(&landing_path_str)
        ),
    )
    .or_panic("write landing");

    let issues = collect_workspace_doc_governance_issues(temp.path(), None);
    let link_issue = issues
        .iter()
        .find(|issue| issue.issue_type == MISSING_PACKAGE_DOCS_INDEX_SECTION_LINK_ISSUE_TYPE)
        .or_panic("missing index section-link issue");
    let expected_insert_offset = format!(
        "# Demo\n\n:PROPERTIES:\n:ID: {}\n:END:\n\n## 01_core: Architecture and Foundation\n\n",
        derive_opaque_doc_id(&index_path_str)
    )
    .len();

    assert_eq!(link_issue.doc, index_path_str);
    assert_eq!(link_issue.severity, "warning");
    assert_eq!(
        link_issue.suggestion.as_deref(),
        Some("- [[01_core/101_demo_core_boundary]]\n")
    );
    assert_eq!(
        link_issue
            .location
            .as_ref()
            .and_then(|location| location.byte_range),
        Some((expected_insert_offset, expected_insert_offset))
    );
}

#[test]
fn detects_missing_standard_index_section_links_before_relations_or_footer_when_heading_missing() {
    let temp = TempDir::new().or_panic("tempdir");
    let crate_dir = temp.path().join("packages/rust/crates/demo");
    let feature_dir = crate_dir.join("docs/03_features");
    fs::create_dir_all(&feature_dir).or_panic("create feature docs dir");
    fs::write(
        crate_dir.join("Cargo.toml"),
        "[package]\nname = \"demo\"\nversion = \"0.1.0\"\n",
    )
    .or_panic("write cargo");

    let index_path = crate_dir.join("docs/index.md");
    let index_path_str = index_path.to_string_lossy().to_string();
    let index_content = format!(
        "# Demo\n\n:PROPERTIES:\n:ID: {}\n:END:\n\n## 01_core: Architecture and Foundation\n\n- [[01_core/101_demo_core_boundary]]\n\n:RELATIONS:\n:LINKS: [[01_core/101_demo_core_boundary]]\n:END:\n\n---\n\n:FOOTER:\n:STANDARDS: v2.0\n:LAST_SYNC: 2026-03-20\n:END:\n",
        derive_opaque_doc_id(&index_path_str)
    );
    fs::write(&index_path, &index_content).or_panic("write index");

    let landing_path = feature_dir.join("201_demo_feature_ledger.md");
    let landing_path_str = landing_path.to_string_lossy().to_string();
    fs::write(
        &landing_path,
        format!(
            "# Feature Ledger\n\n:PROPERTIES:\n:ID: {}\n:END:\n",
            derive_opaque_doc_id(&landing_path_str)
        ),
    )
    .or_panic("write landing");

    let issues = collect_workspace_doc_governance_issues(temp.path(), None);
    let link_issue = issues
        .iter()
        .find(|issue| {
            issue.issue_type == MISSING_PACKAGE_DOCS_INDEX_SECTION_LINK_ISSUE_TYPE
                && issue.message.contains("03_features")
        })
        .or_panic("missing feature section-link issue");

    let relations_offset = index_content
        .find(":RELATIONS:")
        .or_panic("find relations block");

    assert_eq!(link_issue.doc, index_path_str);
    assert_eq!(
        link_issue.suggestion.as_deref(),
        Some("## 03_features\n\n- [[03_features/201_demo_feature_ledger]]\n\n")
    );
    assert_eq!(
        link_issue
            .location
            .as_ref()
            .and_then(|location| location.byte_range),
        Some((relations_offset, relations_offset))
    );
}

#[test]
fn run_audit_core_reports_missing_standard_index_section_links() {
    let temp = TempDir::new().or_panic("tempdir");
    let crate_dir = temp.path().join("packages/rust/crates/demo");
    let core_dir = crate_dir.join("docs/01_core");
    fs::create_dir_all(&core_dir).or_panic("create core docs dir");
    fs::write(
        crate_dir.join("Cargo.toml"),
        "[package]\nname = \"demo\"\nversion = \"0.1.0\"\n",
    )
    .or_panic("write cargo");

    let index_path = crate_dir.join("docs/index.md");
    let index_path_str = index_path.to_string_lossy().to_string();
    fs::write(
        &index_path,
        format!(
            "# Demo\n\n:PROPERTIES:\n:ID: {}\n:END:\n\n",
            derive_opaque_doc_id(&index_path_str)
        ),
    )
    .or_panic("write index");

    let landing_path = core_dir.join("101_demo_core_boundary.md");
    let landing_path_str = landing_path.to_string_lossy().to_string();
    fs::write(
        &landing_path,
        format!(
            "# Core Boundary\n\n:PROPERTIES:\n:ID: {}\n:END:\n",
            derive_opaque_doc_id(&landing_path_str)
        ),
    )
    .or_panic("write landing");

    let index = LinkGraphIndex::build(temp.path()).or_panic("build index");
    let mut ctx = ZhenfaContext::default();
    ctx.insert_extension(index);

    let args = WendaoSemanticCheckArgs {
        doc: Some(".".to_string()),
        checks: Some(vec![CheckType::DocGovernance]),
        include_warnings: Some(true),
        source_paths: None,
        fuzzy_confidence_threshold: None,
    };
    let (issues, _file_contents) = run_audit_core(&ctx, &args).or_panic("audit");

    assert!(issues.iter().any(|issue| {
        issue.issue_type == MISSING_PACKAGE_DOCS_INDEX_SECTION_LINK_ISSUE_TYPE
            && issue
                .doc
                .ends_with("packages/rust/crates/demo/docs/index.md")
    }));
}

#[test]
fn detects_missing_index_relation_links_for_existing_body_links() {
    let temp = TempDir::new().or_panic("tempdir");
    let crate_dir = temp.path().join("packages/rust/crates/demo");
    fs::create_dir_all(crate_dir.join("docs")).or_panic("create docs dir");
    fs::write(
        crate_dir.join("Cargo.toml"),
        "[package]\nname = \"demo\"\nversion = \"0.1.0\"\n",
    )
    .or_panic("write cargo");

    let index_path = crate_dir.join("docs/index.md");
    let index_path_str = index_path.to_string_lossy().to_string();
    let index_content = format!(
        "# Demo\n\n:PROPERTIES:\n:ID: {}\n:END:\n\n## 01_core\n\n- [[01_core/101_demo_core_boundary]]\n- [[01_core/102_demo_contracts]]\n\n:RELATIONS:\n:LINKS: [[01_core/101_demo_core_boundary]]\n:END:\n",
        derive_opaque_doc_id(&index_path_str)
    );
    fs::write(&index_path, &index_content).or_panic("write index");

    let issues = collect_workspace_doc_governance_issues(temp.path(), None);
    let relation_issue = issues
        .iter()
        .find(|issue| issue.issue_type == MISSING_PACKAGE_DOCS_INDEX_RELATION_LINK_ISSUE_TYPE)
        .or_panic("missing relation-link issue");

    assert_eq!(relation_issue.doc, index_path_str);
    assert_eq!(relation_issue.severity, "warning");
    assert!(
        relation_issue
            .message
            .contains("[[01_core/102_demo_contracts]]")
    );
    assert_eq!(
        relation_issue.suggestion.as_deref(),
        Some("[[01_core/101_demo_core_boundary]], [[01_core/102_demo_contracts]]")
    );
    let links_value = "[[01_core/101_demo_core_boundary]]";
    let links_line_start = index_content.find(":LINKS: ").or_panic("find links line");
    let value_start = links_line_start
        + ":LINKS: ".len()
        + index_content[links_line_start + ":LINKS: ".len()..]
            .find(links_value)
            .or_panic("find relation links value");
    let value_end = value_start + links_value.len();
    assert_eq!(
        relation_issue
            .location
            .as_ref()
            .and_then(|location| location.byte_range),
        Some((value_start, value_end))
    );
}

#[test]
fn detects_missing_index_relations_block_for_existing_body_links() {
    let temp = TempDir::new().or_panic("tempdir");
    let crate_dir = temp.path().join("packages/rust/crates/demo");
    fs::create_dir_all(crate_dir.join("docs")).or_panic("create docs dir");
    fs::write(
        crate_dir.join("Cargo.toml"),
        "[package]\nname = \"demo\"\nversion = \"0.1.0\"\n",
    )
    .or_panic("write cargo");

    let index_path = crate_dir.join("docs/index.md");
    let index_path_str = index_path.to_string_lossy().to_string();
    let index_content = format!(
        "# Demo\n\n:PROPERTIES:\n:ID: {}\n:END:\n\n## 01_core\n\n- [[01_core/101_demo_core_boundary]]\n\n---\n\n:FOOTER:\n:STANDARDS: v2.0\n:END:\n",
        derive_opaque_doc_id(&index_path_str)
    );
    fs::write(&index_path, &index_content).or_panic("write index");

    let issues = collect_workspace_doc_governance_issues(temp.path(), None);
    let block_issue = issues
        .iter()
        .find(|issue| issue.issue_type == MISSING_PACKAGE_DOCS_INDEX_RELATIONS_BLOCK_ISSUE_TYPE)
        .or_panic("missing relations-block issue");

    assert_eq!(block_issue.doc, index_path_str);
    assert_eq!(block_issue.severity, "warning");
    assert!(
        block_issue
            .message
            .contains("[[01_core/101_demo_core_boundary]]")
    );
    assert_eq!(
        block_issue.suggestion.as_deref(),
        Some(":RELATIONS:\n:LINKS: [[01_core/101_demo_core_boundary]]\n:END:\n\n")
    );
    let insert_offset = index_content.find("---").or_panic("find footer separator");
    assert_eq!(
        block_issue
            .location
            .as_ref()
            .and_then(|location| location.byte_range),
        Some((insert_offset, insert_offset))
    );
}

#[test]
fn detects_missing_index_footer_block_for_existing_relations_block() {
    let temp = TempDir::new().or_panic("tempdir");
    let crate_dir = temp.path().join("packages/rust/crates/demo");
    fs::create_dir_all(crate_dir.join("docs")).or_panic("create docs dir");
    fs::write(
        crate_dir.join("Cargo.toml"),
        "[package]\nname = \"demo\"\nversion = \"0.1.0\"\n",
    )
    .or_panic("write cargo");

    let index_path = crate_dir.join("docs/index.md");
    let index_path_str = index_path.to_string_lossy().to_string();
    let index_content = format!(
        "# Demo\n\n:PROPERTIES:\n:ID: {}\n:END:\n\n## 01_core\n\n- [[01_core/101_demo_core_boundary]]\n\n:RELATIONS:\n:LINKS: [[01_core/101_demo_core_boundary]]\n:END:\n",
        derive_opaque_doc_id(&index_path_str)
    );
    fs::write(&index_path, &index_content).or_panic("write index");

    let issues = collect_workspace_doc_governance_issues(temp.path(), None);
    let footer_issue = issues
        .iter()
        .find(|issue| issue.issue_type == MISSING_PACKAGE_DOCS_INDEX_FOOTER_BLOCK_ISSUE_TYPE)
        .or_panic("missing footer-block issue");

    assert_eq!(footer_issue.doc, index_path_str);
    assert_eq!(footer_issue.severity, "warning");
    assert!(footer_issue.message.contains(":FOOTER:"));
    assert_eq!(
        footer_issue.suggestion.as_deref(),
        Some("\n---\n\n:FOOTER:\n:STANDARDS: v2.0\n:LAST_SYNC: pending\n:END:\n")
    );
    assert_eq!(
        footer_issue
            .location
            .as_ref()
            .and_then(|location| location.byte_range),
        Some((index_content.len(), index_content.len()))
    );
}

#[test]
fn detects_incomplete_index_footer_block_for_existing_footer() {
    let temp = TempDir::new().or_panic("tempdir");
    let crate_dir = temp.path().join("packages/rust/crates/demo");
    fs::create_dir_all(crate_dir.join("docs")).or_panic("create docs dir");
    fs::write(
        crate_dir.join("Cargo.toml"),
        "[package]\nname = \"demo\"\nversion = \"0.1.0\"\n",
    )
    .or_panic("write cargo");

    let index_path = crate_dir.join("docs/index.md");
    let index_path_str = index_path.to_string_lossy().to_string();
    let footer_block = ":FOOTER:\n:STANDARDS: v2.0\n:END:\n";
    let index_content = format!(
        "# Demo\n\n:PROPERTIES:\n:ID: {}\n:END:\n\n## 01_core\n\n- [[01_core/101_demo_core_boundary]]\n\n:RELATIONS:\n:LINKS: [[01_core/101_demo_core_boundary]]\n:END:\n\n---\n\n{footer_block}",
        derive_opaque_doc_id(&index_path_str)
    );
    fs::write(&index_path, &index_content).or_panic("write index");

    let issues = collect_workspace_doc_governance_issues(temp.path(), None);
    let footer_issue = issues
        .iter()
        .find(|issue| issue.issue_type == INCOMPLETE_PACKAGE_DOCS_INDEX_FOOTER_BLOCK_ISSUE_TYPE)
        .or_panic("missing incomplete footer-block issue");

    let footer_start = index_content.find(":FOOTER:").or_panic("find footer start");
    let footer_end = footer_start + footer_block.len();

    assert_eq!(footer_issue.doc, index_path_str);
    assert_eq!(footer_issue.severity, "warning");
    assert!(footer_issue.message.contains(":LAST_SYNC:"));
    assert_eq!(
        footer_issue.suggestion.as_deref(),
        Some(":FOOTER:\n:STANDARDS: v2.0\n:LAST_SYNC: pending\n:END:\n")
    );
    assert_eq!(
        footer_issue
            .location
            .as_ref()
            .and_then(|location| location.byte_range),
        Some((footer_start, footer_end))
    );
}

#[test]
fn detects_stale_index_footer_standards_for_existing_footer() {
    let temp = TempDir::new().or_panic("tempdir");
    let crate_dir = temp.path().join("packages/rust/crates/demo");
    fs::create_dir_all(crate_dir.join("docs")).or_panic("create docs dir");
    fs::write(
        crate_dir.join("Cargo.toml"),
        "[package]\nname = \"demo\"\nversion = \"0.1.0\"\n",
    )
    .or_panic("write cargo");

    let index_path = crate_dir.join("docs/index.md");
    let index_path_str = index_path.to_string_lossy().to_string();
    let footer_block = ":FOOTER:\n:STANDARDS: v1.0\n:LAST_SYNC: 2026-03-20\n:END:\n";
    let index_content = format!(
        "# Demo\n\n:PROPERTIES:\n:ID: {}\n:END:\n\n## 01_core\n\n- [[01_core/101_demo_core_boundary]]\n\n:RELATIONS:\n:LINKS: [[01_core/101_demo_core_boundary]]\n:END:\n\n---\n\n{footer_block}",
        derive_opaque_doc_id(&index_path_str)
    );
    fs::write(&index_path, &index_content).or_panic("write index");

    let issues = collect_workspace_doc_governance_issues(temp.path(), None);
    let footer_issue = issues
        .iter()
        .find(|issue| issue.issue_type == STALE_PACKAGE_DOCS_INDEX_FOOTER_STANDARDS_ISSUE_TYPE)
        .or_panic("missing stale footer-standards issue");

    let footer_start = index_content.find(":FOOTER:").or_panic("find footer start");
    let footer_end = footer_start + footer_block.len();

    assert_eq!(footer_issue.doc, index_path_str);
    assert_eq!(footer_issue.severity, "warning");
    assert!(footer_issue.message.contains("v1.0"));
    assert_eq!(
        footer_issue.suggestion.as_deref(),
        Some(":FOOTER:\n:STANDARDS: v2.0\n:LAST_SYNC: 2026-03-20\n:END:\n")
    );
    assert_eq!(
        footer_issue
            .location
            .as_ref()
            .and_then(|location| location.byte_range),
        Some((footer_start, footer_end))
    );
}

#[test]
fn run_audit_core_reports_missing_index_relation_links() {
    let temp = TempDir::new().or_panic("tempdir");
    let crate_dir = temp.path().join("packages/rust/crates/demo");
    fs::create_dir_all(crate_dir.join("docs")).or_panic("create docs dir");
    fs::write(
        crate_dir.join("Cargo.toml"),
        "[package]\nname = \"demo\"\nversion = \"0.1.0\"\n",
    )
    .or_panic("write cargo");

    let index_path = crate_dir.join("docs/index.md");
    let index_path_str = index_path.to_string_lossy().to_string();
    fs::write(
        &index_path,
        format!(
            "# Demo\n\n:PROPERTIES:\n:ID: {}\n:END:\n\n## 01_core\n\n- [[01_core/101_demo_core_boundary]]\n\n:RELATIONS:\n:LINKS: \n:END:\n",
            derive_opaque_doc_id(&index_path_str)
        ),
    )
    .or_panic("write index");

    let index = LinkGraphIndex::build(temp.path()).or_panic("build index");
    let mut ctx = ZhenfaContext::default();
    ctx.insert_extension(index);

    let args = WendaoSemanticCheckArgs {
        doc: Some(".".to_string()),
        checks: Some(vec![CheckType::DocGovernance]),
        include_warnings: Some(true),
        source_paths: None,
        fuzzy_confidence_threshold: None,
    };
    let (issues, _file_contents) = run_audit_core(&ctx, &args).or_panic("audit");

    assert!(issues.iter().any(|issue| {
        issue.issue_type == MISSING_PACKAGE_DOCS_INDEX_RELATION_LINK_ISSUE_TYPE
            && issue
                .doc
                .ends_with("packages/rust/crates/demo/docs/index.md")
    }));
}

#[test]
fn detects_stale_index_relation_links_without_missing_links() {
    let temp = TempDir::new().or_panic("tempdir");
    let crate_dir = temp.path().join("packages/rust/crates/demo");
    fs::create_dir_all(crate_dir.join("docs")).or_panic("create docs dir");
    fs::write(
        crate_dir.join("Cargo.toml"),
        "[package]\nname = \"demo\"\nversion = \"0.1.0\"\n",
    )
    .or_panic("write cargo");

    let index_path = crate_dir.join("docs/index.md");
    let index_path_str = index_path.to_string_lossy().to_string();
    let index_content = format!(
        "# Demo\n\n:PROPERTIES:\n:ID: {}\n:END:\n\n## 01_core\n\n- [[01_core/101_demo_core_boundary]]\n\n:RELATIONS:\n:LINKS: [[01_core/101_demo_core_boundary]], [[01_core/999_stale]]\n:END:\n",
        derive_opaque_doc_id(&index_path_str)
    );
    fs::write(&index_path, &index_content).or_panic("write index");

    let issues = collect_workspace_doc_governance_issues(temp.path(), None);
    let stale_issue = issues
        .iter()
        .find(|issue| issue.issue_type == STALE_PACKAGE_DOCS_INDEX_RELATION_LINK_ISSUE_TYPE)
        .or_panic("missing stale relation-link issue");

    assert_eq!(stale_issue.doc, index_path_str);
    assert_eq!(stale_issue.severity, "warning");
    assert!(stale_issue.message.contains("[[01_core/999_stale]]"));
    assert_eq!(
        stale_issue.suggestion.as_deref(),
        Some("[[01_core/101_demo_core_boundary]]")
    );
    let relation_value = "[[01_core/101_demo_core_boundary]], [[01_core/999_stale]]";
    let links_line_start = index_content.find(":LINKS: ").or_panic("find links line");
    let value_start = links_line_start
        + ":LINKS: ".len()
        + index_content[links_line_start + ":LINKS: ".len()..]
            .find(relation_value)
            .or_panic("find stale relation links value");
    let value_end = value_start + relation_value.len();
    assert_eq!(
        stale_issue
            .location
            .as_ref()
            .and_then(|location| location.byte_range),
        Some((value_start, value_end))
    );
}

#[test]
fn run_audit_core_reports_stale_index_relation_links() {
    let temp = TempDir::new().or_panic("tempdir");
    let crate_dir = temp.path().join("packages/rust/crates/demo");
    fs::create_dir_all(crate_dir.join("docs")).or_panic("create docs dir");
    fs::write(
        crate_dir.join("Cargo.toml"),
        "[package]\nname = \"demo\"\nversion = \"0.1.0\"\n",
    )
    .or_panic("write cargo");

    let index_path = crate_dir.join("docs/index.md");
    let index_path_str = index_path.to_string_lossy().to_string();
    fs::write(
        &index_path,
        format!(
            "# Demo\n\n:PROPERTIES:\n:ID: {}\n:END:\n\n## 01_core\n\n- [[01_core/101_demo_core_boundary]]\n\n:RELATIONS:\n:LINKS: [[01_core/101_demo_core_boundary]], [[01_core/999_stale]]\n:END:\n",
            derive_opaque_doc_id(&index_path_str)
        ),
    )
    .or_panic("write index");

    let index = LinkGraphIndex::build(temp.path()).or_panic("build index");
    let mut ctx = ZhenfaContext::default();
    ctx.insert_extension(index);

    let args = WendaoSemanticCheckArgs {
        doc: Some(".".to_string()),
        checks: Some(vec![CheckType::DocGovernance]),
        include_warnings: Some(true),
        source_paths: None,
        fuzzy_confidence_threshold: None,
    };
    let (issues, _file_contents) = run_audit_core(&ctx, &args).or_panic("audit");

    assert!(issues.iter().any(|issue| {
        issue.issue_type == STALE_PACKAGE_DOCS_INDEX_RELATION_LINK_ISSUE_TYPE
            && issue
                .doc
                .ends_with("packages/rust/crates/demo/docs/index.md")
    }));
}

#[test]
fn run_audit_core_reports_missing_index_footer_block() {
    let temp = TempDir::new().or_panic("tempdir");
    let crate_dir = temp.path().join("packages/rust/crates/demo");
    fs::create_dir_all(crate_dir.join("docs")).or_panic("create docs dir");
    fs::write(
        crate_dir.join("Cargo.toml"),
        "[package]\nname = \"demo\"\nversion = \"0.1.0\"\n",
    )
    .or_panic("write cargo");

    let index_path = crate_dir.join("docs/index.md");
    let index_path_str = index_path.to_string_lossy().to_string();
    fs::write(
        &index_path,
        format!(
            "# Demo\n\n:PROPERTIES:\n:ID: {}\n:END:\n\n## 01_core\n\n- [[01_core/101_demo_core_boundary]]\n\n:RELATIONS:\n:LINKS: [[01_core/101_demo_core_boundary]]\n:END:\n",
            derive_opaque_doc_id(&index_path_str)
        ),
    )
    .or_panic("write index");

    let index = LinkGraphIndex::build(temp.path()).or_panic("build index");
    let mut ctx = ZhenfaContext::default();
    ctx.insert_extension(index);

    let args = WendaoSemanticCheckArgs {
        doc: Some(".".to_string()),
        checks: Some(vec![CheckType::DocGovernance]),
        include_warnings: Some(true),
        source_paths: None,
        fuzzy_confidence_threshold: None,
    };
    let (issues, _file_contents) = run_audit_core(&ctx, &args).or_panic("audit");

    assert!(issues.iter().any(|issue| {
        issue.issue_type == MISSING_PACKAGE_DOCS_INDEX_FOOTER_BLOCK_ISSUE_TYPE
            && issue
                .doc
                .ends_with("packages/rust/crates/demo/docs/index.md")
    }));
}

#[test]
fn run_audit_core_reports_incomplete_index_footer_block() {
    let temp = TempDir::new().or_panic("tempdir");
    let crate_dir = temp.path().join("packages/rust/crates/demo");
    fs::create_dir_all(crate_dir.join("docs")).or_panic("create docs dir");
    fs::write(
        crate_dir.join("Cargo.toml"),
        "[package]\nname = \"demo\"\nversion = \"0.1.0\"\n",
    )
    .or_panic("write cargo");

    let index_path = crate_dir.join("docs/index.md");
    let index_path_str = index_path.to_string_lossy().to_string();
    fs::write(
        &index_path,
        format!(
            "# Demo\n\n:PROPERTIES:\n:ID: {}\n:END:\n\n## 01_core\n\n- [[01_core/101_demo_core_boundary]]\n\n:RELATIONS:\n:LINKS: [[01_core/101_demo_core_boundary]]\n:END:\n\n---\n\n:FOOTER:\n:STANDARDS: v2.0\n:END:\n",
            derive_opaque_doc_id(&index_path_str)
        ),
    )
    .or_panic("write index");

    let index = LinkGraphIndex::build(temp.path()).or_panic("build index");
    let mut ctx = ZhenfaContext::default();
    ctx.insert_extension(index);

    let args = WendaoSemanticCheckArgs {
        doc: Some(".".to_string()),
        checks: Some(vec![CheckType::DocGovernance]),
        include_warnings: Some(true),
        source_paths: None,
        fuzzy_confidence_threshold: None,
    };
    let (issues, _file_contents) = run_audit_core(&ctx, &args).or_panic("audit");

    assert!(issues.iter().any(|issue| {
        issue.issue_type == INCOMPLETE_PACKAGE_DOCS_INDEX_FOOTER_BLOCK_ISSUE_TYPE
            && issue
                .doc
                .ends_with("packages/rust/crates/demo/docs/index.md")
    }));
}

#[test]
fn run_audit_core_reports_stale_index_footer_standards() {
    let temp = TempDir::new().or_panic("tempdir");
    let crate_dir = temp.path().join("packages/rust/crates/demo");
    fs::create_dir_all(crate_dir.join("docs")).or_panic("create docs dir");
    fs::write(
        crate_dir.join("Cargo.toml"),
        "[package]\nname = \"demo\"\nversion = \"0.1.0\"\n",
    )
    .or_panic("write cargo");

    let index_path = crate_dir.join("docs/index.md");
    let index_path_str = index_path.to_string_lossy().to_string();
    fs::write(
        &index_path,
        format!(
            "# Demo\n\n:PROPERTIES:\n:ID: {}\n:END:\n\n## 01_core\n\n- [[01_core/101_demo_core_boundary]]\n\n:RELATIONS:\n:LINKS: [[01_core/101_demo_core_boundary]]\n:END:\n\n---\n\n:FOOTER:\n:STANDARDS: v1.0\n:LAST_SYNC: 2026-03-20\n:END:\n",
            derive_opaque_doc_id(&index_path_str)
        ),
    )
    .or_panic("write index");

    let index = LinkGraphIndex::build(temp.path()).or_panic("build index");
    let mut ctx = ZhenfaContext::default();
    ctx.insert_extension(index);

    let args = WendaoSemanticCheckArgs {
        doc: Some(".".to_string()),
        checks: Some(vec![CheckType::DocGovernance]),
        include_warnings: Some(true),
        source_paths: None,
        fuzzy_confidence_threshold: None,
    };
    let (issues, _file_contents) = run_audit_core(&ctx, &args).or_panic("audit");

    assert!(issues.iter().any(|issue| {
        issue.issue_type == STALE_PACKAGE_DOCS_INDEX_FOOTER_STANDARDS_ISSUE_TYPE
            && issue
                .doc
                .ends_with("packages/rust/crates/demo/docs/index.md")
    }));
}

#[test]
fn run_audit_core_loads_explicit_workspace_doc_file_for_fix_generation() {
    let temp = TempDir::new().or_panic("tempdir");
    let crate_dir = temp.path().join("packages/rust/crates/demo");
    fs::create_dir_all(crate_dir.join("docs")).or_panic("create docs dir");
    fs::write(
        crate_dir.join("Cargo.toml"),
        "[package]\nname = \"demo\"\nversion = \"0.1.0\"\n",
    )
    .or_panic("write cargo");

    let index_path = crate_dir.join("docs/index.md");
    let index_path_str = index_path.to_string_lossy().to_string();
    fs::write(
        &index_path,
        format!(
            "# Demo\n\n:PROPERTIES:\n:ID: {}\n:END:\n\n## 01_core\n\n- [[01_core/101_demo_core_boundary]]\n\n:RELATIONS:\n:LINKS: [[01_core/101_demo_core_boundary]]\n:END:\n\n---\n\n:FOOTER:\n:STANDARDS: v1.0\n:LAST_SYNC: 2026-03-20\n:END:\n",
            derive_opaque_doc_id(&index_path_str)
        ),
    )
    .or_panic("write index");

    let index = LinkGraphIndex::build(temp.path()).or_panic("build index");
    let mut ctx = ZhenfaContext::default();
    ctx.insert_extension(index);

    let args = WendaoSemanticCheckArgs {
        doc: Some(index_path_str.clone()),
        checks: Some(vec![CheckType::DocGovernance]),
        include_warnings: Some(true),
        source_paths: None,
        fuzzy_confidence_threshold: None,
    };
    let (_issues, file_contents) = run_audit_core(&ctx, &args).or_panic("audit");

    assert!(file_contents.contains_key(&index_path_str));

    let content = file_contents.get(&index_path_str).or_panic("missing value");
    let issues = collect_stale_index_footer_standards(&index_path_str, content);
    assert_eq!(issues.len(), 1);

    let fixes = generate_surgical_fixes(&issues, &file_contents);
    assert_eq!(fixes.len(), 1);
    assert_eq!(
        fixes[0].issue_type,
        STALE_PACKAGE_DOCS_INDEX_FOOTER_STANDARDS_ISSUE_TYPE
    );
}

#[test]
fn run_audit_core_reports_doc_identity_for_explicit_workspace_doc_file() {
    let temp = TempDir::new().or_panic("tempdir");
    let crate_dir = temp.path().join("packages/rust/crates/demo");
    fs::create_dir_all(crate_dir.join("docs")).or_panic("create docs dir");
    fs::write(
        crate_dir.join("Cargo.toml"),
        "[package]\nname = \"demo\"\nversion = \"0.1.0\"\n",
    )
    .or_panic("write cargo");

    let index_path = crate_dir.join("docs/index.md");
    let index_path_str = index_path.to_string_lossy().to_string();
    fs::write(
        &index_path,
        "# Demo\n\n:PROPERTIES:\n:ID: readable-demo-index\n:TYPE: INDEX\n:END:\n",
    )
    .or_panic("write index");

    let index = LinkGraphIndex::build(temp.path()).or_panic("build index");
    let mut ctx = ZhenfaContext::default();
    ctx.insert_extension(index);

    let args = WendaoSemanticCheckArgs {
        doc: Some(index_path_str.clone()),
        checks: Some(vec![CheckType::DocGovernance]),
        include_warnings: Some(true),
        source_paths: None,
        fuzzy_confidence_threshold: None,
    };
    let (issues, _file_contents) = run_audit_core(&ctx, &args).or_panic("audit");

    let issue = issues
        .iter()
        .find(|issue| issue.issue_type == DOC_IDENTITY_PROTOCOL_ISSUE_TYPE)
        .or_panic("doc identity issue");
    let canonical_index_path = index_path
        .canonicalize()
        .or_panic("canonical index path")
        .to_string_lossy()
        .to_string();
    let expected_id = derive_opaque_doc_id(&canonical_index_path);
    assert_eq!(issue.doc, canonical_index_path);
    assert_eq!(issue.severity, "error");
    assert_eq!(issue.suggestion.as_deref(), Some(expected_id.as_str()));
}

#[test]
fn run_audit_core_seeds_workspace_doc_identity_issue_files_for_fix_generation() {
    let temp = TempDir::new().or_panic("tempdir");
    let crate_dir = temp.path().join("packages/rust/crates/demo");
    let core_dir = crate_dir.join("docs/01_core");
    fs::create_dir_all(&core_dir).or_panic("create core docs dir");
    fs::write(
        crate_dir.join("Cargo.toml"),
        "[package]\nname = \"demo\"\nversion = \"0.1.0\"\n",
    )
    .or_panic("write cargo");

    let index_path = crate_dir.join("docs/index.md");
    let index_path_str = index_path.to_string_lossy().to_string();
    fs::write(
        &index_path,
        format!(
            "# Demo\n\n:PROPERTIES:\n:ID: {}\n:TYPE: INDEX\n:END:\n",
            derive_opaque_doc_id(&index_path_str)
        ),
    )
    .or_panic("write index");

    let intro_path = core_dir.join("101_intro.md");
    fs::write(
        &intro_path,
        "# Intro\n\n:PROPERTIES:\n:ID: readable-intro\n:TYPE: CORE\n:END:\n",
    )
    .or_panic("write intro");

    let index = LinkGraphIndex::build(temp.path()).or_panic("build index");
    let mut ctx = ZhenfaContext::default();
    ctx.insert_extension(index);

    let docs_scope = crate_dir.join("docs").to_string_lossy().to_string();
    let args = WendaoSemanticCheckArgs {
        doc: Some(docs_scope),
        checks: Some(vec![CheckType::DocGovernance]),
        include_warnings: Some(true),
        source_paths: None,
        fuzzy_confidence_threshold: None,
    };
    let (issues, file_contents) = run_audit_core(&ctx, &args).or_panic("audit");
    let canonical_intro_path = intro_path
        .canonicalize()
        .or_panic("canonical intro path")
        .to_string_lossy()
        .to_string();

    let identity_issue = issues
        .iter()
        .find(|issue| {
            issue.issue_type == DOC_IDENTITY_PROTOCOL_ISSUE_TYPE
                && issue.doc == canonical_intro_path
        })
        .or_panic("workspace doc identity issue");

    assert!(file_contents.contains_key(&canonical_intro_path));

    let fixes = generate_surgical_fixes(std::slice::from_ref(identity_issue), &file_contents);
    assert_eq!(fixes.len(), 1);
    assert_eq!(fixes[0].issue_type, DOC_IDENTITY_PROTOCOL_ISSUE_TYPE);
}

#[test]
fn package_docs_directory_scope_fix_rewrites_doc_identity_issues_end_to_end() {
    let temp = TempDir::new().or_panic("tempdir");
    let crate_dir = temp.path().join("packages/rust/crates/demo");
    let core_dir = crate_dir.join("docs/01_core");
    let feature_dir = crate_dir.join("docs/03_features");
    fs::create_dir_all(&core_dir).or_panic("create core docs dir");
    fs::create_dir_all(&feature_dir).or_panic("create feature docs dir");
    fs::write(
        crate_dir.join("Cargo.toml"),
        "[package]\nname = \"demo\"\nversion = \"0.1.0\"\n",
    )
    .or_panic("write cargo");

    let index_path = crate_dir.join("docs/index.md");
    let index_path_str = index_path.to_string_lossy().to_string();
    fs::write(
        &index_path,
        format!(
            "# Demo\n\n:PROPERTIES:\n:ID: {}\n:TYPE: INDEX\n:END:\n",
            derive_opaque_doc_id(&index_path_str)
        ),
    )
    .or_panic("write index");

    let core_doc = core_dir.join("101_intro.md");
    let core_doc_str = core_doc.to_string_lossy().to_string();
    fs::write(
        &core_doc,
        "# Intro\n\n:PROPERTIES:\n:ID: readable-intro\n:TYPE: CORE\n:END:\n",
    )
    .or_panic("write core doc");

    let feature_doc = feature_dir.join("201_feature_ledger.md");
    let feature_doc_str = feature_doc.to_string_lossy().to_string();
    fs::write(
        &feature_doc,
        "# Feature Ledger\n\n:PROPERTIES:\n:ID: readable-feature-ledger\n:TYPE: FEATURE\n:END:\n",
    )
    .or_panic("write feature doc");

    let index = LinkGraphIndex::build(temp.path()).or_panic("build index");
    let mut ctx = ZhenfaContext::default();
    ctx.insert_extension(index);

    let args = WendaoSemanticCheckArgs {
        doc: Some(crate_dir.join("docs").to_string_lossy().to_string()),
        checks: Some(vec![CheckType::DocGovernance]),
        include_warnings: Some(true),
        source_paths: None,
        fuzzy_confidence_threshold: None,
    };
    let (issues, file_contents) = run_audit_core(&ctx, &args).or_panic("audit");

    let doc_identity_issues = issues
        .iter()
        .filter(|issue| issue.issue_type == DOC_IDENTITY_PROTOCOL_ISSUE_TYPE)
        .cloned()
        .collect::<Vec<_>>();
    assert_eq!(doc_identity_issues.len(), 2);

    let fixes = generate_surgical_fixes(&doc_identity_issues, &file_contents);
    assert_eq!(fixes.len(), 2);

    let report = AtomicFixBatch::new(fixes).apply_all();
    assert!(report.is_success(), "{}", report.summary());

    let core_doc_content = fs::read_to_string(&core_doc).or_panic("read core doc");
    assert!(core_doc_content.contains(&format!(":ID: {}", derive_opaque_doc_id(&core_doc_str))));

    let feature_doc_content = fs::read_to_string(&feature_doc).or_panic("read feature doc");
    assert!(
        feature_doc_content.contains(&format!(":ID: {}", derive_opaque_doc_id(&feature_doc_str)))
    );
}

#[test]
fn run_audit_core_reports_missing_index_relations_block() {
    let temp = TempDir::new().or_panic("tempdir");
    let crate_dir = temp.path().join("packages/rust/crates/demo");
    fs::create_dir_all(crate_dir.join("docs")).or_panic("create docs dir");
    fs::write(
        crate_dir.join("Cargo.toml"),
        "[package]\nname = \"demo\"\nversion = \"0.1.0\"\n",
    )
    .or_panic("write cargo");

    let index_path = crate_dir.join("docs/index.md");
    let index_path_str = index_path.to_string_lossy().to_string();
    fs::write(
        &index_path,
        format!(
            "# Demo\n\n:PROPERTIES:\n:ID: {}\n:END:\n\n## 01_core\n\n- [[01_core/101_demo_core_boundary]]\n",
            derive_opaque_doc_id(&index_path_str)
        ),
    )
    .or_panic("write index");

    let index = LinkGraphIndex::build(temp.path()).or_panic("build index");
    let mut ctx = ZhenfaContext::default();
    ctx.insert_extension(index);

    let args = WendaoSemanticCheckArgs {
        doc: Some(".".to_string()),
        checks: Some(vec![CheckType::DocGovernance]),
        include_warnings: Some(true),
        source_paths: None,
        fuzzy_confidence_threshold: None,
    };
    let (issues, _file_contents) = run_audit_core(&ctx, &args).or_panic("audit");

    assert!(issues.iter().any(|issue| {
        issue.issue_type == MISSING_PACKAGE_DOCS_INDEX_RELATIONS_BLOCK_ISSUE_TYPE
            && issue
                .doc
                .ends_with("packages/rust/crates/demo/docs/index.md")
    }));
}
