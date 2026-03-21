use crate::analyzers::{RepoSymbolKind, RepositoryAnalysisOutput};
use crate::gateway::studio::types::{
    CodeAstAnalysisResponse, CodeAstEdge, CodeAstEdgeKind, CodeAstNode, CodeAstNodeKind,
    CodeAstProjection, CodeAstProjectionKind,
};

/// Build the code-AST response payload for one repository-relative source path.
pub fn build_code_ast_analysis_response(
    repo_id: String,
    path: String,
    line_hint: Option<usize>,
    analysis: &RepositoryAnalysisOutput,
) -> CodeAstAnalysisResponse {
    let mut nodes = Vec::new();
    let mut edges = Vec::new();
    let mut contains_edge_count = 0usize;
    let mut uses_edge_count = 0usize;
    let mut interaction_edge_count = 0usize;

    // Convert modules to nodes
    for module in &analysis.modules {
        nodes.push(CodeAstNode {
            id: module.module_id.clone(),
            label: module.qualified_name.clone(),
            kind: CodeAstNodeKind::Module,
            path: Some(module.path.clone()),
            line: Some(1),
        });
    }

    // Convert symbols to nodes
    for symbol in &analysis.symbols {
        let kind = if repo_relative_path_matches(symbol.path.as_str(), path.as_str()) {
            match symbol.kind {
                RepoSymbolKind::Function => CodeAstNodeKind::Function,
                RepoSymbolKind::Type => CodeAstNodeKind::Type,
                RepoSymbolKind::Constant => CodeAstNodeKind::Constant,
                _ => CodeAstNodeKind::Other,
            }
        } else {
            CodeAstNodeKind::ExternalSymbol
        };
        nodes.push(CodeAstNode {
            id: symbol.symbol_id.clone(),
            label: symbol.name.clone(),
            kind,
            path: Some(symbol.path.clone()),
            line: symbol.line_start,
        });
    }

    // Convert relations to edges
    for relation in &analysis.relations {
        let kind = match relation.kind {
            crate::analyzers::RelationKind::Contains => {
                contains_edge_count += 1;
                CodeAstEdgeKind::Contains
            }
            crate::analyzers::RelationKind::Calls => {
                interaction_edge_count += 1;
                CodeAstEdgeKind::Calls
            }
            crate::analyzers::RelationKind::Uses => {
                interaction_edge_count += 1;
                uses_edge_count += 1;
                CodeAstEdgeKind::Uses
            }
            crate::analyzers::RelationKind::Imports => {
                interaction_edge_count += 1;
                CodeAstEdgeKind::Imports
            }
            _ => CodeAstEdgeKind::Other,
        };
        edges.push(CodeAstEdge {
            id: format!(
                "{}-{}-{}",
                relation.source_id, relation.target_id, relation.kind as u8
            ),
            source_id: relation.source_id.clone(),
            target_id: relation.target_id.clone(),
            kind,
            label: None,
        });
    }

    let language = if path.ends_with(".jl") {
        "julia"
    } else {
        "modelica"
    };
    let focus_node_id = line_hint
        .and_then(|line| {
            analysis.symbols.iter().find(|symbol| {
                if !repo_relative_path_matches(symbol.path.as_str(), path.as_str()) {
                    return false;
                }
                match (symbol.line_start, symbol.line_end) {
                    (Some(start), Some(end)) => start <= line && line <= end,
                    (Some(start), None) => start == line,
                    _ => false,
                }
            })
        })
        .or_else(|| {
            line_hint.and_then(|_| {
                analysis
                    .symbols
                    .iter()
                    .find(|symbol| repo_relative_path_matches(symbol.path.as_str(), path.as_str()))
            })
        })
        .map(|symbol| symbol.symbol_id.clone());
    let projections = vec![
        CodeAstProjection {
            kind: CodeAstProjectionKind::Contains,
            node_count: nodes.len(),
            edge_count: contains_edge_count,
        },
        CodeAstProjection {
            kind: CodeAstProjectionKind::Calls,
            node_count: nodes.len(),
            edge_count: interaction_edge_count,
        },
        CodeAstProjection {
            kind: CodeAstProjectionKind::Uses,
            node_count: nodes.len(),
            edge_count: uses_edge_count,
        },
    ];

    CodeAstAnalysisResponse {
        repo_id,
        path,
        language: language.to_string(),
        nodes,
        edges,
        projections,
        focus_node_id,
        diagnostics: Vec::new(),
    }
}

/// Resolve the repository context and normalized repository-relative path for code-AST queries.
pub fn resolve_code_ast_repository_and_path<'a>(
    repositories: &'a [crate::analyzers::RegisteredRepository],
    repo_id: Option<&str>,
    path: &str,
) -> Result<
    (&'a crate::analyzers::RegisteredRepository, String),
    crate::gateway::studio::router::StudioApiError,
> {
    if let Some(id) = repo_id {
        let repo = repositories.iter().find(|r| r.id == id).ok_or_else(|| {
            crate::gateway::studio::router::StudioApiError::bad_request(
                "UNKNOWN_REPO",
                format!("Repository `{id}` not found"),
            )
        })?;
        return Ok((repo, path.to_string()));
    }

    // Heuristic: try to find repo id in path prefix
    for repo in repositories {
        let prefix = format!("{}/", repo.id);
        if path.starts_with(&prefix) {
            return Ok((repo, path[prefix.len()..].to_string()));
        }
    }

    Err(crate::gateway::studio::router::StudioApiError::bad_request(
        "MISSING_REPO",
        "Repository context is required",
    ))
}

fn repo_relative_path_matches(path: &str, target: &str) -> bool {
    path == target || path.ends_with(&format!("/{}", target))
}
