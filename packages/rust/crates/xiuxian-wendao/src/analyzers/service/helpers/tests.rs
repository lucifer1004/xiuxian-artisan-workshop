use std::collections::{BTreeMap, BTreeSet};

use crate::analyzers::plugin::RepositoryAnalysisOutput;
use crate::analyzers::query::RepoBacklinkItem;
use crate::analyzers::records::{
    DocRecord, ExampleRecord, ImportKind, ImportRecord, ModuleRecord, RelationKind, RelationRecord,
    RepoSymbolKind, RepositoryRecord, SymbolRecord,
};

use super::{
    backlinks_for, docs_in_scope, documented_symbol_ids, documents_backlink_lookup,
    example_match_score, example_relation_lookup, hierarchy_segments_from_path, import_match_score,
    infer_ecosystem, module_match_score, normalized_rank_score, projection_page_lookup,
    projection_pages_for, record_hierarchical_uri, related_modules_for_example,
    related_symbols_for_example, relation_kind_label, repo_hierarchical_uri, resolve_module_scope,
    symbol_match_score, symbols_in_scope,
};

fn some_or_panic<T>(value: Option<T>, context: &str) -> T {
    value.unwrap_or_else(|| panic!("{context}"))
}

fn repository_record(repo_id: &str) -> RepositoryRecord {
    RepositoryRecord {
        repo_id: repo_id.to_string(),
        name: repo_id.to_string(),
        ..RepositoryRecord::default()
    }
}

fn module_record(repo_id: &str, module_id: &str, qualified_name: &str, path: &str) -> ModuleRecord {
    ModuleRecord {
        repo_id: repo_id.to_string(),
        module_id: module_id.to_string(),
        qualified_name: qualified_name.to_string(),
        path: path.to_string(),
    }
}

fn symbol_record(
    repo_id: &str,
    symbol_id: &str,
    module_id: Option<&str>,
    name: &str,
    qualified_name: &str,
    path: &str,
) -> SymbolRecord {
    SymbolRecord {
        repo_id: repo_id.to_string(),
        symbol_id: symbol_id.to_string(),
        module_id: module_id.map(str::to_string),
        name: name.to_string(),
        qualified_name: qualified_name.to_string(),
        kind: RepoSymbolKind::Function,
        path: path.to_string(),
        line_start: None,
        line_end: None,
        signature: Some(format!("fn {name}()")),
        audit_status: None,
        verification_state: None,
        attributes: BTreeMap::new(),
    }
}

fn doc_record(repo_id: &str, doc_id: &str, title: &str, path: &str) -> DocRecord {
    DocRecord {
        repo_id: repo_id.to_string(),
        doc_id: doc_id.to_string(),
        title: title.to_string(),
        path: path.to_string(),
        format: Some("markdown".to_string()),
    }
}

fn example_record(repo_id: &str, example_id: &str, title: &str, path: &str) -> ExampleRecord {
    ExampleRecord {
        repo_id: repo_id.to_string(),
        example_id: example_id.to_string(),
        title: title.to_string(),
        path: path.to_string(),
        summary: Some(format!("{title} summary")),
    }
}

fn import_record(
    repo_id: &str,
    module_id: &str,
    import_name: &str,
    target_package: &str,
    source_module: &str,
) -> ImportRecord {
    ImportRecord {
        repo_id: repo_id.to_string(),
        module_id: module_id.to_string(),
        import_name: import_name.to_string(),
        target_package: target_package.to_string(),
        source_module: source_module.to_string(),
        kind: ImportKind::Module,
        resolved_id: None,
    }
}

fn analysis_fixture() -> RepositoryAnalysisOutput {
    let repo_id = "repo-a";
    RepositoryAnalysisOutput {
        repository: Some(repository_record(repo_id)),
        modules: vec![
            module_record(repo_id, "mod-a", "alpha.beta", "src/alpha/beta.rs"),
            module_record(repo_id, "mod-b", "omega.gamma", "src/omega/gamma.rs"),
        ],
        symbols: vec![
            symbol_record(
                repo_id,
                "sym-a",
                Some("mod-a"),
                "Solve",
                "alpha.beta::Solve",
                "src/alpha/beta.rs#solve",
            ),
            symbol_record(
                repo_id,
                "sym-b",
                Some("mod-b"),
                "Helper",
                "omega.gamma::Helper",
                "src/omega/gamma.rs#helper",
            ),
        ],
        imports: vec![import_record(
            repo_id,
            "mod-a",
            "solver",
            "sciml-solver",
            "alpha.beta",
        )],
        examples: vec![
            example_record(repo_id, "ex-a", "Solve Example", "examples/solve.rs"),
            example_record(repo_id, "ex-b", "Helper Example", "examples/helper.rs"),
        ],
        docs: vec![
            doc_record(repo_id, "doc-a", "Alpha Guide", "docs/alpha.md"),
            doc_record(repo_id, "doc-b", "Symbol Guide", "docs/symbol.md"),
        ],
        relations: vec![
            RelationRecord {
                repo_id: repo_id.to_string(),
                source_id: "doc-a".to_string(),
                target_id: "mod-a".to_string(),
                kind: RelationKind::Documents,
            },
            RelationRecord {
                repo_id: repo_id.to_string(),
                source_id: "doc-b".to_string(),
                target_id: "sym-a".to_string(),
                kind: RelationKind::Documents,
            },
            RelationRecord {
                repo_id: repo_id.to_string(),
                source_id: "doc-b".to_string(),
                target_id: "sym-a".to_string(),
                kind: RelationKind::Documents,
            },
            RelationRecord {
                repo_id: repo_id.to_string(),
                source_id: "ex-a".to_string(),
                target_id: "sym-a".to_string(),
                kind: RelationKind::ExampleOf,
            },
            RelationRecord {
                repo_id: repo_id.to_string(),
                source_id: "ex-a".to_string(),
                target_id: "mod-a".to_string(),
                kind: RelationKind::ExampleOf,
            },
            RelationRecord {
                repo_id: repo_id.to_string(),
                source_id: "ex-a".to_string(),
                target_id: "sym-a".to_string(),
                kind: RelationKind::ExampleOf,
            },
        ],
        diagnostics: Vec::new(),
    }
}

#[test]
fn relation_labels_and_uris_remain_stable() {
    let expected = [
        (RelationKind::Contains, "contains"),
        (RelationKind::Calls, "calls"),
        (RelationKind::Uses, "uses"),
        (RelationKind::Documents, "documents"),
        (RelationKind::ExampleOf, "example_of"),
        (RelationKind::Declares, "declares"),
        (RelationKind::Implements, "implements"),
        (RelationKind::Imports, "imports"),
    ];

    for (kind, label) in expected {
        assert_eq!(relation_kind_label(kind), label);
    }

    assert_eq!(repo_hierarchical_uri("repo-a"), "repo://repo-a");
    assert_eq!(
        record_hierarchical_uri("repo-a", "sciml", "symbol", "/src/alpha/", "sym-a"),
        "wendao://repo/sciml/repo-a/symbol/src:alpha/sym-a"
    );
}

#[test]
fn ecosystem_and_path_helpers_cover_common_inputs() {
    assert_eq!(infer_ecosystem("Diffeq-Docs"), "sciml");
    assert_eq!(infer_ecosystem("MSL"), "msl");
    assert_eq!(infer_ecosystem("plain-repo"), "unknown");
    assert_eq!(
        hierarchy_segments_from_path("/alpha//beta/"),
        Some(vec!["alpha".to_string(), "beta".to_string()])
    );
}

#[test]
fn ranking_helpers_distinguish_common_match_shapes() {
    assert!((normalized_rank_score(0, 3) - 1.0).abs() < f64::EPSILON);
    assert!((normalized_rank_score(3, 3) - 0.25).abs() < f64::EPSILON);
    assert_eq!(
        module_match_score("alpha", "alpha.beta", "src/alpha/beta.rs"),
        Some(1)
    );
    assert_eq!(
        symbol_match_score(
            "solve",
            "solve",
            "alpha.beta::solve",
            "src/alpha/beta.rs",
            "fn solve()"
        ),
        Some(0)
    );
    assert_eq!(
        example_match_score(
            "solve",
            "solve example",
            "examples/solve.rs",
            "solve summary",
            &[String::from("related-symbol")],
            &[String::from("related-module")],
        ),
        Some(1)
    );
    assert_eq!(
        import_match_score(
            Some("sciml-solver"),
            Some("alpha.beta"),
            &import_record("repo-a", "mod-a", "solver", "sciml-solver", "alpha.beta")
        ),
        Some(0)
    );
}

#[test]
fn backlinks_and_example_relations_are_deduplicated_and_trimmed() {
    let analysis = analysis_fixture();
    let backlink_lookup = documents_backlink_lookup(&analysis.relations, &analysis.docs);
    let (backlink_ids, backlink_items) = backlinks_for("mod-a", &backlink_lookup);
    assert_eq!(backlink_ids, Some(vec!["doc-a".to_string()]));
    assert_eq!(
        backlink_items,
        Some(vec![RepoBacklinkItem {
            id: "doc-a".to_string(),
            title: Some("Alpha Guide".to_string()),
            path: Some("docs/alpha.md".to_string()),
            kind: Some("documents".to_string()),
        }])
    );

    let relation_lookup = example_relation_lookup(&analysis.relations);
    let related_symbols = related_symbols_for_example("ex-a", &relation_lookup, &analysis.symbols);
    let related_modules = related_modules_for_example("ex-a", &relation_lookup, &analysis.modules);
    assert_eq!(
        related_symbols,
        vec!["solve".to_string(), "alpha.beta::solve".to_string()]
    );
    assert_eq!(
        related_modules,
        vec!["alpha.beta".to_string(), "beta".to_string()]
    );
}

#[test]
fn scope_helpers_filter_docs_symbols_and_modules() {
    let analysis = analysis_fixture();
    let scoped_module = some_or_panic(
        resolve_module_scope(Some("alpha.beta"), &analysis.modules),
        "module scope should resolve by qualified name",
    );
    assert_eq!(scoped_module.module_id, "mod-a");
    assert_eq!(
        some_or_panic(
            resolve_module_scope(Some("src/alpha/beta.rs"), &analysis.modules),
            "module scope should resolve by path",
        )
        .module_id,
        "mod-a"
    );

    let scoped_symbols = symbols_in_scope(Some(scoped_module), &analysis.symbols);
    assert_eq!(
        scoped_symbols
            .iter()
            .map(|symbol| symbol.symbol_id.as_str())
            .collect::<Vec<_>>(),
        vec!["sym-a"]
    );

    let scoped_docs = docs_in_scope(Some(scoped_module), &analysis);
    assert_eq!(
        scoped_docs
            .iter()
            .map(|doc| doc.doc_id.as_str())
            .collect::<Vec<_>>(),
        vec!["doc-a", "doc-b"]
    );

    let documented =
        documented_symbol_ids(Some(scoped_module), &analysis.symbols, &analysis.relations);
    assert_eq!(documented, BTreeSet::from([String::from("sym-a")]));
}

#[test]
fn projection_lookup_collects_page_ids_for_each_anchor() {
    let analysis = analysis_fixture();
    let lookup = projection_page_lookup(&analysis);

    assert!(projection_pages_for("mod-a", &lookup).is_some());
    assert!(projection_pages_for("sym-a", &lookup).is_some());
    assert!(projection_pages_for("ex-a", &lookup).is_some());
    assert!(projection_pages_for("doc-a", &lookup).is_some());
}
