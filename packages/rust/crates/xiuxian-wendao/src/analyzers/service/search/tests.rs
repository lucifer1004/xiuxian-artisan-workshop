use std::collections::BTreeMap;

use super::{build_example_search, build_module_search, build_symbol_search};
use crate::analyzers::query::{ExampleSearchQuery, ModuleSearchQuery, SymbolSearchQuery};
use crate::analyzers::{
    DocRecord, ExampleRecord, ModuleRecord, RelationKind, RelationRecord, RepoSymbolKind,
    RepositoryAnalysisOutput, RepositoryRecord, SymbolRecord,
};

#[allow(clippy::too_many_lines)]
fn sample_search_analysis(repo_id: &str) -> RepositoryAnalysisOutput {
    let module_id = format!("repo:{repo_id}:module:ProjectionPkg");
    let solve_symbol_id = format!("repo:{repo_id}:symbol:ProjectionPkg.solve");
    let problem_symbol_id = format!("repo:{repo_id}:symbol:ProjectionPkg.Problem");
    let readme_doc_id = format!("repo:{repo_id}:doc:README.md");
    let solve_doc_id = format!("repo:{repo_id}:doc:src/ProjectionPkg.jl#symbol:solve");
    let problem_doc_id = format!("repo:{repo_id}:doc:src/ProjectionPkg.jl#symbol:Problem");
    let example_id = format!("repo:{repo_id}:example:examples/basic.jl");

    RepositoryAnalysisOutput {
        repository: Some(RepositoryRecord {
            repo_id: repo_id.to_string(),
            name: "ProjectionPkg".to_string(),
            path: format!("/virtual/repos/{repo_id}"),
            url: None,
            revision: Some("fixture".to_string()),
            version: Some("0.1.0".to_string()),
            uuid: None,
            dependencies: Vec::new(),
        }),
        modules: vec![ModuleRecord {
            repo_id: repo_id.to_string(),
            module_id: module_id.clone(),
            qualified_name: "ProjectionPkg".to_string(),
            path: "src/ProjectionPkg.jl".to_string(),
        }],
        symbols: vec![
            SymbolRecord {
                repo_id: repo_id.to_string(),
                symbol_id: solve_symbol_id.clone(),
                module_id: Some(module_id.clone()),
                name: "solve".to_string(),
                qualified_name: "ProjectionPkg.solve".to_string(),
                kind: RepoSymbolKind::Function,
                path: "src/ProjectionPkg.jl".to_string(),
                line_start: None,
                line_end: None,
                signature: Some("solve(problem::Problem)".to_string()),
                audit_status: None,
                verification_state: None,
                attributes: BTreeMap::new(),
            },
            SymbolRecord {
                repo_id: repo_id.to_string(),
                symbol_id: problem_symbol_id.clone(),
                module_id: Some(module_id.clone()),
                name: "Problem".to_string(),
                qualified_name: "ProjectionPkg.Problem".to_string(),
                kind: RepoSymbolKind::Type,
                path: "src/ProjectionPkg.jl".to_string(),
                line_start: None,
                line_end: None,
                signature: Some("struct Problem".to_string()),
                audit_status: None,
                verification_state: None,
                attributes: BTreeMap::new(),
            },
        ],
        imports: Vec::new(),
        examples: vec![ExampleRecord {
            repo_id: repo_id.to_string(),
            example_id: example_id.clone(),
            title: "basic".to_string(),
            path: "examples/basic.jl".to_string(),
            summary: Some("Solve a projection problem end to end.".to_string()),
        }],
        docs: vec![
            DocRecord {
                repo_id: repo_id.to_string(),
                doc_id: readme_doc_id.clone(),
                title: "README.md".to_string(),
                path: "README.md".to_string(),
                format: Some("md".to_string()),
            },
            DocRecord {
                repo_id: repo_id.to_string(),
                doc_id: problem_doc_id.clone(),
                title: "Problem".to_string(),
                path: "src/ProjectionPkg.jl#symbol:Problem".to_string(),
                format: Some("julia_docstring".to_string()),
            },
            DocRecord {
                repo_id: repo_id.to_string(),
                doc_id: solve_doc_id.clone(),
                title: "solve".to_string(),
                path: "src/ProjectionPkg.jl#symbol:solve".to_string(),
                format: Some("julia_docstring".to_string()),
            },
        ],
        relations: vec![
            RelationRecord {
                repo_id: repo_id.to_string(),
                source_id: readme_doc_id,
                target_id: module_id.clone(),
                kind: RelationKind::Documents,
            },
            RelationRecord {
                repo_id: repo_id.to_string(),
                source_id: problem_doc_id,
                target_id: problem_symbol_id,
                kind: RelationKind::Documents,
            },
            RelationRecord {
                repo_id: repo_id.to_string(),
                source_id: solve_doc_id,
                target_id: solve_symbol_id.clone(),
                kind: RelationKind::Documents,
            },
            RelationRecord {
                repo_id: repo_id.to_string(),
                source_id: example_id.clone(),
                target_id: module_id.clone(),
                kind: RelationKind::ExampleOf,
            },
            RelationRecord {
                repo_id: repo_id.to_string(),
                source_id: example_id,
                target_id: solve_symbol_id,
                kind: RelationKind::ExampleOf,
            },
        ],
        diagnostics: Vec::new(),
    }
}

#[test]
fn module_search_uses_shared_tantivy_fuzzy_index_for_typos() {
    let analysis = sample_search_analysis("module-fuzzy");
    let result = build_module_search(
        &ModuleSearchQuery {
            repo_id: "module-fuzzy".to_string(),
            query: "ProjectonPkg".to_string(),
            limit: 10,
        },
        &analysis,
    );

    assert_eq!(result.modules.len(), 1);
    assert_eq!(result.modules[0].qualified_name, "ProjectionPkg");
    assert!(
        result.module_hits[0]
            .score
            .unwrap_or_else(|| panic!("shared fuzzy module search should emit a score"))
            > 0.0
    );
}

#[test]
fn symbol_search_uses_shared_tantivy_fuzzy_index_for_typos() {
    let analysis = sample_search_analysis("symbol-fuzzy");
    let result = build_symbol_search(
        &SymbolSearchQuery {
            repo_id: "symbol-fuzzy".to_string(),
            query: "slove".to_string(),
            limit: 10,
        },
        &analysis,
    );

    assert_eq!(result.symbols.len(), 1);
    assert_eq!(result.symbols[0].name, "solve");
    assert!(
        result.symbol_hits[0]
            .score
            .unwrap_or_else(|| panic!("shared fuzzy symbol search should emit a score"))
            > 0.0
    );
}

#[test]
fn example_search_uses_shared_tantivy_fuzzy_index_for_related_symbol_typos() {
    let analysis = sample_search_analysis("example-fuzzy");
    let result = build_example_search(
        &ExampleSearchQuery {
            repo_id: "example-fuzzy".to_string(),
            query: "slove".to_string(),
            limit: 10,
        },
        &analysis,
    );

    assert_eq!(result.examples.len(), 1);
    assert_eq!(result.examples[0].title, "basic");
    assert!(
        result.example_hits[0]
            .score
            .unwrap_or_else(|| panic!("shared fuzzy example search should emit a score"))
            > 0.0
    );
}
