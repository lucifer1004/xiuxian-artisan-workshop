use crate::gateway::studio::router::StudioState;
use crate::gateway::studio::types::{AnalysisNode, MarkdownAnalysisResponse};

use super::markdown::{CompiledDocument, compile_markdown_ir};
use super::projection;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AnalysisError {
    UnsupportedContentType(String),
    Vfs(String),
}

impl std::fmt::Display for AnalysisError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnsupportedContentType(content_type) => {
                write!(f, "unsupported content type: {content_type}")
            }
            Self::Vfs(message) => write!(f, "{message}"),
        }
    }
}

impl std::error::Error for AnalysisError {}

pub(crate) fn compile_markdown_nodes(path: &str, content: &str) -> Vec<AnalysisNode> {
    compile_markdown_ir(path, content).nodes
}

pub(crate) async fn analyze_markdown(
    state: &StudioState,
    path: &str,
) -> Result<MarkdownAnalysisResponse, AnalysisError> {
    if !is_markdown_path(path) {
        return Err(AnalysisError::UnsupportedContentType(
            infer_content_type(path).to_string(),
        ));
    }

    let project_root = state.project_root.clone();
    let full_path = project_root.join(path);

    let content = std::fs::read_to_string(&full_path)
        .map_err(|e| AnalysisError::Vfs(format!("Failed to read file: {e}")))?;

    let compiled: CompiledDocument = compile_markdown_ir(path, &content);

    // Optional: add link-graph metadata if index is available
    let _index = match state.graph_index.read() {
        Ok(guard) => guard.as_ref().map(std::sync::Arc::clone),
        Err(_) => None,
    };

    let projections = projection::build_mermaid_projections(&compiled.nodes, &compiled.edges);

    Ok(MarkdownAnalysisResponse {
        path: path.to_string(),
        document_hash: compiled.document_hash,
        node_count: compiled.nodes.len(),
        edge_count: compiled.edges.len(),
        nodes: compiled.nodes,
        edges: compiled.edges,
        projections,
        diagnostics: compiled.diagnostics,
    })
}

fn is_markdown_path(path: &str) -> bool {
    path.ends_with(".md") || path.ends_with(".markdown")
}

fn infer_content_type(path: &str) -> &'static str {
    if path.ends_with(".rs") {
        "text/x-rust"
    } else if is_markdown_path(path) {
        "text/markdown"
    } else {
        "application/octet-stream"
    }
}
