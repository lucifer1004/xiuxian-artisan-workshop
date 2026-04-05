use std::collections::BTreeMap;

use crate::analyzers::{
    ModuleRecord, RelationKind, RelationRecord, RepoSymbolKind, RepositoryAnalysisOutput,
    SymbolRecord,
};
use crate::gateway::studio::router::build_code_ast_analysis_response;
use crate::gateway::studio::types::{
    CodeAstAnalysisResponse, CodeAstEdgeKind, CodeAstNodeKind, CodeAstProjectionKind,
    CodeAstRetrievalAtomScope,
};

#[test]
fn build_code_ast_analysis_response_emits_uses_projection_and_external_node() {
    let payload = build_code_ast_analysis_response(
        "sciml".to_string(),
        "src/BaseModelica.jl".to_string(),
        Some(7),
        Some(sample_source()),
        &sample_analysis(),
    );

    assert_code_ast_summary(&payload);
    assert_code_ast_retrieval_atoms(&payload);
}

fn sample_analysis() -> RepositoryAnalysisOutput {
    RepositoryAnalysisOutput {
        modules: vec![ModuleRecord {
            repo_id: "sciml".to_string(),
            module_id: "module:BaseModelica".to_string(),
            qualified_name: "BaseModelica".to_string(),
            path: "src/BaseModelica.jl".to_string(),
        }],
        symbols: vec![
            SymbolRecord {
                repo_id: "sciml".to_string(),
                symbol_id: "symbol:reexport".to_string(),
                module_id: Some("module:BaseModelica".to_string()),
                name: "reexport".to_string(),
                qualified_name: "BaseModelica.reexport".to_string(),
                kind: RepoSymbolKind::Function,
                path: "src/BaseModelica.jl".to_string(),
                line_start: Some(7),
                line_end: Some(9),
                signature: None,
                audit_status: None,
                verification_state: None,
                attributes: BTreeMap::new(),
            },
            SymbolRecord {
                repo_id: "sciml".to_string(),
                symbol_id: "symbol:ModelicaSystem".to_string(),
                module_id: None,
                name: "ModelicaSystem".to_string(),
                qualified_name: "ModelicaSystem".to_string(),
                kind: RepoSymbolKind::Type,
                path: "src/modelica/system.jl".to_string(),
                line_start: Some(1),
                line_end: Some(3),
                signature: None,
                audit_status: None,
                verification_state: None,
                attributes: BTreeMap::new(),
            },
        ],
        relations: vec![RelationRecord {
            repo_id: "sciml".to_string(),
            source_id: "symbol:reexport".to_string(),
            target_id: "symbol:ModelicaSystem".to_string(),
            kind: RelationKind::Uses,
        }],
        ..RepositoryAnalysisOutput::default()
    }
}

fn sample_source() -> &'static str {
    "module BaseModelica\n\
\n\
# prelude\n\
# prelude\n\
# prelude\n\
# prelude\n\
pub fn reexport(\n\
    input,\n\
) {\n\
    if isempty(input)\n\
        return Err(Empty)\n\
    end\n\
\n\
    let meta = parse(input)\n\
\n\
    return Ok(meta)\n\
}\n"
}

fn assert_code_ast_summary(payload: &CodeAstAnalysisResponse) {
    assert_eq!(payload.language, "julia");
    assert!(
        payload
            .nodes
            .iter()
            .any(|node| matches!(node.kind, CodeAstNodeKind::ExternalSymbol))
    );
    assert!(
        payload
            .edges
            .iter()
            .any(|edge| matches!(edge.kind, CodeAstEdgeKind::Uses))
    );
    assert!(payload.projections.iter().any(|projection| {
        matches!(projection.kind, CodeAstProjectionKind::Calls) && projection.edge_count > 0
    }));
    assert!(payload.focus_node_id.is_some());
}

fn assert_code_ast_retrieval_atoms(payload: &CodeAstAnalysisResponse) {
    assert!(payload.retrieval_atoms.iter().any(|atom| {
        atom.owner_id == "symbol:reexport"
            && matches!(atom.surface, Some(CodeAstRetrievalAtomScope::Declaration))
            && atom
                .chunk_id
                .starts_with("ast:src-basemodelica-jl:declaration:function:")
            && atom.token_estimate > 0
    }));
    assert!(payload.retrieval_atoms.iter().any(|atom| {
        atom.owner_id == "symbol:ModelicaSystem"
            && matches!(atom.surface, Some(CodeAstRetrievalAtomScope::Symbol))
            && atom
                .chunk_id
                .starts_with("ast:src-modelica-system-jl:symbol:externalsymbol:")
            && atom.fingerprint.starts_with("fp:")
    }));
    assert!(payload.retrieval_atoms.iter().any(|atom| {
        atom.owner_id.starts_with("block:validation:")
            && matches!(atom.surface, Some(CodeAstRetrievalAtomScope::Block))
            && atom.semantic_type == "validation"
            && atom.line_start.is_some()
            && atom.line_end >= atom.line_start
    }));
    assert!(payload.retrieval_atoms.iter().any(|atom| {
        atom.owner_id.starts_with("block:return:")
            && matches!(atom.surface, Some(CodeAstRetrievalAtomScope::Block))
            && atom.semantic_type == "return"
            && atom.line_start.is_some()
            && atom.line_end >= atom.line_start
    }));
}
