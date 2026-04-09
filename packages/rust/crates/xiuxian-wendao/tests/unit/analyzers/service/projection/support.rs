use std::collections::BTreeMap;

use crate::analyzers::{
    DocRecord, ModuleRecord, RelationKind, RelationRecord, RepoSymbolKind,
    RepositoryAnalysisOutput, RepositoryRecord, SymbolRecord,
};

#[must_use]
#[allow(clippy::too_many_lines)]
pub fn sample_projection_analysis(repo_id: &str) -> RepositoryAnalysisOutput {
    let module_id = format!("repo:{repo_id}:module:ProjectionPkg");
    let solve_symbol_id = format!("repo:{repo_id}:symbol:ProjectionPkg.solve");
    let problem_symbol_id = format!("repo:{repo_id}:symbol:ProjectionPkg.Problem");
    let readme_doc_id = format!("repo:{repo_id}:doc:README.md");
    let solve_doc_id = format!("repo:{repo_id}:doc:src/ProjectionPkg.jl#symbol:solve");
    let problem_doc_id = format!("repo:{repo_id}:doc:src/ProjectionPkg.jl#symbol:Problem");

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
        examples: Vec::new(),
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
                target_id: solve_symbol_id,
                kind: RelationKind::Documents,
            },
        ],
        diagnostics: Vec::new(),
    }
}
