use std::collections::BTreeMap;
use std::path::Path;

use xiuxian_ast::{
    JuliaDocAttachment, JuliaDocTargetKind, JuliaImport, JuliaParseError, JuliaSourceSummary,
    JuliaSymbol, JuliaSymbolKind as AstJuliaSymbolKind, TreeSitterJuliaParser,
};
use xiuxian_wendao_core::repo_intelligence::{
    AnalysisContext, DocRecord, ModuleRecord, PluginAnalysisOutput, PluginLinkContext,
    RegisteredRepository, RelationKind, RelationRecord, RepoIntelligenceError,
    RepoIntelligencePlugin, RepoSourceFile, RepoSymbolKind, RepositoryAnalysisOutput,
    RepositoryRecord, SymbolRecord,
};

use super::discovery::{discover_docs, discover_examples, relative_path_string};
use super::graph_structural::{
    GraphStructuralRouteKind,
};
use super::linking::{build_doc_relations, build_example_relations};
use super::project::{load_project_metadata, locate_root_module_file};
use super::sources::{JuliaAnalyzedFile, collect_julia_sources};
use super::transport::build_julia_flight_transport_client;
use super::graph_structural_transport::build_graph_structural_flight_transport_client;

const JULIA_PLUGIN_ID: &str = "julia";

/// External Julia analyzer for Repo Intelligence.
#[derive(Debug, Default, Clone, Copy)]
pub struct JuliaRepoIntelligencePlugin;

/// Register the Julia plugin into an existing Repo Intelligence registry.
///
/// # Errors
///
/// Returns [`RepoIntelligenceError`] when the registry already contains a
/// plugin with the `julia` identifier.
pub fn register_into(
    registry: &mut xiuxian_wendao_core::repo_intelligence::PluginRegistry,
) -> Result<(), RepoIntelligenceError> {
    registry.register(JuliaRepoIntelligencePlugin)
}

impl RepoIntelligencePlugin for JuliaRepoIntelligencePlugin {
    fn id(&self) -> &'static str {
        JULIA_PLUGIN_ID
    }

    fn supports_repository(&self, repository: &RegisteredRepository) -> bool {
        repository
            .plugins
            .iter()
            .any(|plugin| plugin.id() == JULIA_PLUGIN_ID)
    }

    fn analyze_file(
        &self,
        context: &AnalysisContext,
        file: &RepoSourceFile,
    ) -> Result<PluginAnalysisOutput, RepoIntelligenceError> {
        let summary = parse_julia_source_summary(&file.contents, &file.path)?;
        let module_id = format!(
            "repo:{}:module:{}",
            context.repository.id, summary.module_name
        );
        let symbols = build_symbol_records(
            &context.repository.id,
            &file.path,
            &summary.module_name,
            &summary.exports,
            &summary.symbols,
        );
        Ok(PluginAnalysisOutput {
            modules: vec![ModuleRecord {
                repo_id: context.repository.id.clone(),
                module_id,
                qualified_name: summary.module_name.clone(),
                path: file.path.clone(),
            }],
            symbols: symbols.clone(),
            docs: build_docstring_records(
                &context.repository.id,
                &file.path,
                &summary.module_name,
                &symbols,
                &summary.docstrings,
            ),
            examples: Vec::new(),
            diagnostics: Vec::new(),
        })
    }

    fn preflight_repository(
        &self,
        context: &AnalysisContext,
        repository_root: &Path,
    ) -> Result<(), RepoIntelligenceError> {
        let _maybe_transport = build_julia_flight_transport_client(&context.repository)?;
        let _maybe_graph_structural_rerank_transport = build_graph_structural_flight_transport_client(
            &context.repository,
            GraphStructuralRouteKind::StructuralRerank,
        )?;
        let _maybe_graph_structural_filter_transport = build_graph_structural_flight_transport_client(
            &context.repository,
            GraphStructuralRouteKind::ConstraintFilter,
        )?;
        let metadata = load_project_metadata(&context.repository.id, repository_root)?;
        let mut diagnostics = Vec::new();
        let _ = locate_root_module_file(
            &context.repository.id,
            repository_root,
            &metadata.name,
            &mut diagnostics,
        )?;
        Ok(())
    }

    fn analyze_repository(
        &self,
        context: &AnalysisContext,
        repository_root: &Path,
    ) -> Result<RepositoryAnalysisOutput, RepoIntelligenceError> {
        let metadata = load_project_metadata(&context.repository.id, repository_root)?;
        let mut diagnostics = Vec::new();
        let root_file = locate_root_module_file(
            &context.repository.id,
            repository_root,
            &metadata.name,
            &mut diagnostics,
        )?;
        let root_path = relative_path_string(repository_root, &root_file)?;
        let collected = collect_julia_sources(
            &context.repository.id,
            repository_root,
            &root_file,
            &mut diagnostics,
        )?;
        let module_id = format!(
            "repo:{}:module:{}",
            context.repository.id, collected.root_summary.module_name
        );
        let examples = discover_examples(&context.repository.id, repository_root)?;
        let modules = vec![ModuleRecord {
            repo_id: context.repository.id.clone(),
            module_id: module_id.clone(),
            qualified_name: collected.root_summary.module_name.clone(),
            path: root_path.clone(),
        }];
        let symbols = collect_symbol_records(
            &context.repository.id,
            &collected.root_summary.module_name,
            &collected.files,
        );
        let mut docs = discover_docs(&context.repository.id, repository_root)?;
        for file in &collected.files {
            docs.extend(build_docstring_records(
                &context.repository.id,
                &file.path,
                &collected.root_summary.module_name,
                &symbols,
                &file.summary.docstrings,
            ));
        }
        let mut relations = Vec::new();
        for file in &collected.files {
            relations.extend(build_import_relations(
                &context.repository.id,
                &module_id,
                &file.summary.imports,
            ));
            relations.extend(build_docstring_relations(
                &context.repository.id,
                &module_id,
                &symbols,
                &file.summary.docstrings,
                &file.path,
            ));
        }
        relations.extend(build_structural_relations(
            &context.repository.id,
            &modules,
            &symbols,
            &examples,
            &docs,
        ));

        Ok(RepositoryAnalysisOutput {
            repository: Some(RepositoryRecord {
                repo_id: context.repository.id.clone(),
                name: metadata.name,
                path: repository_root.display().to_string(),
                url: context.repository.url.clone(),
                revision: None,
                version: metadata.version,
                uuid: metadata.uuid,
                dependencies: metadata.dependencies,
            }),
            modules,
            symbols,
            imports: Vec::new(),
            examples,
            docs,
            relations,
            diagnostics,
        })
    }

    fn enrich_relations(
        &self,
        context: &PluginLinkContext,
    ) -> Result<Vec<RelationRecord>, RepoIntelligenceError> {
        let mut relations = build_doc_relations(context)?;
        relations.extend(build_example_relations(context)?);
        Ok(relations)
    }
}

fn parse_julia_source_summary(
    contents: &str,
    root_path: &str,
) -> Result<JuliaSourceSummary, RepoIntelligenceError> {
    let mut parser =
        TreeSitterJuliaParser::new().map_err(map_julia_parse_error(root_path.to_string()))?;
    parser
        .parse_summary(contents)
        .map_err(map_julia_parse_error(root_path.to_string()))
}

fn collect_symbol_records(
    repo_id: &str,
    module_name: &str,
    files: &[JuliaAnalyzedFile],
) -> Vec<SymbolRecord> {
    let mut symbol_map = BTreeMap::new();

    for file in files {
        for symbol in build_symbol_records(
            repo_id,
            &file.path,
            module_name,
            &file.summary.exports,
            &file.summary.symbols,
        ) {
            upsert_symbol(&mut symbol_map, symbol);
        }
    }

    symbol_map.into_values().collect()
}

fn map_julia_parse_error(
    root_path: String,
) -> impl FnOnce(JuliaParseError) -> RepoIntelligenceError {
    move |error| RepoIntelligenceError::AnalysisFailed {
        message: format!("failed to parse Julia source `{root_path}`: {error}"),
    }
}

fn build_symbol_records(
    repo_id: &str,
    path: &str,
    module_name: &str,
    exports: &[String],
    symbols: &[JuliaSymbol],
) -> Vec<SymbolRecord> {
    let mut symbol_map = BTreeMap::new();

    for export_name in exports {
        let symbol = build_symbol_record(
            repo_id,
            path,
            module_name,
            export_name,
            RepoSymbolKind::ModuleExport,
            None,
        );
        symbol_map.entry(symbol.symbol_id.clone()).or_insert(symbol);
    }

    for symbol in symbols {
        let kind = match symbol.kind {
            AstJuliaSymbolKind::Function => RepoSymbolKind::Function,
            AstJuliaSymbolKind::Type => RepoSymbolKind::Type,
        };
        let record = build_symbol_record(
            repo_id,
            path,
            module_name,
            &symbol.name,
            kind,
            symbol.signature.clone(),
        );
        upsert_symbol(&mut symbol_map, record);
    }

    symbol_map.into_values().collect()
}

fn build_import_relations(
    repo_id: &str,
    module_id: &str,
    imports: &[JuliaImport],
) -> Vec<RelationRecord> {
    imports
        .iter()
        .map(|import| RelationRecord {
            repo_id: repo_id.to_string(),
            source_id: module_id.to_string(),
            target_id: format!("external:{}", import.module),
            kind: RelationKind::Uses,
        })
        .collect()
}

fn build_structural_relations(
    repo_id: &str,
    modules: &[ModuleRecord],
    symbols: &[SymbolRecord],
    examples: &[xiuxian_wendao_core::repo_intelligence::ExampleRecord],
    docs: &[DocRecord],
) -> Vec<RelationRecord> {
    let repository_node_id = format!("repo:{repo_id}");
    let mut relations = Vec::new();

    relations.extend(modules.iter().map(|module| RelationRecord {
        repo_id: repo_id.to_string(),
        source_id: repository_node_id.clone(),
        target_id: module.module_id.clone(),
        kind: RelationKind::Contains,
    }));
    relations.extend(symbols.iter().filter_map(|symbol| {
        symbol.module_id.as_ref().map(|module_id| RelationRecord {
            repo_id: repo_id.to_string(),
            source_id: module_id.clone(),
            target_id: symbol.symbol_id.clone(),
            kind: RelationKind::Declares,
        })
    }));
    relations.extend(examples.iter().map(|example| RelationRecord {
        repo_id: repo_id.to_string(),
        source_id: repository_node_id.clone(),
        target_id: example.example_id.clone(),
        kind: RelationKind::Contains,
    }));
    relations.extend(docs.iter().map(|doc| RelationRecord {
        repo_id: repo_id.to_string(),
        source_id: repository_node_id.clone(),
        target_id: doc.doc_id.clone(),
        kind: RelationKind::Contains,
    }));

    relations
}

fn build_docstring_records(
    repo_id: &str,
    path: &str,
    module_name: &str,
    symbols: &[SymbolRecord],
    docstrings: &[JuliaDocAttachment],
) -> Vec<DocRecord> {
    docstrings
        .iter()
        .filter_map(|docstring| {
            let anchor = match docstring.target_kind {
                JuliaDocTargetKind::Module if docstring.target_name == module_name => {
                    format!("module:{}", docstring.target_name)
                }
                JuliaDocTargetKind::Symbol => symbols
                    .iter()
                    .find(|symbol| symbol.name == docstring.target_name)
                    .map(|_| format!("symbol:{}", docstring.target_name))?,
                JuliaDocTargetKind::Module => return None,
            };
            Some(DocRecord {
                repo_id: repo_id.to_string(),
                doc_id: format!("repo:{repo_id}:doc:{path}#{anchor}"),
                title: docstring.target_name.clone(),
                path: format!("{path}#{anchor}"),
                format: Some("julia_docstring".to_string()),
            })
        })
        .collect()
}

fn build_docstring_relations(
    repo_id: &str,
    module_id: &str,
    symbols: &[SymbolRecord],
    docstrings: &[JuliaDocAttachment],
    path: &str,
) -> Vec<RelationRecord> {
    docstrings
        .iter()
        .filter_map(|docstring| {
            let (anchor, target_id) = match docstring.target_kind {
                JuliaDocTargetKind::Module => (
                    format!("module:{}", docstring.target_name),
                    module_id.to_string(),
                ),
                JuliaDocTargetKind::Symbol => (
                    format!("symbol:{}", docstring.target_name),
                    symbols
                        .iter()
                        .find(|symbol| symbol.name == docstring.target_name)
                        .map(|symbol| symbol.symbol_id.clone())?,
                ),
            };
            Some(RelationRecord {
                repo_id: repo_id.to_string(),
                source_id: format!("repo:{repo_id}:doc:{path}#{anchor}"),
                target_id,
                kind: RelationKind::Documents,
            })
        })
        .collect()
}

fn build_symbol_record(
    repo_id: &str,
    path: &str,
    module_name: &str,
    symbol_name: &str,
    kind: RepoSymbolKind,
    signature: Option<String>,
) -> SymbolRecord {
    let qualified_name = format!("{module_name}.{symbol_name}");
    SymbolRecord {
        repo_id: repo_id.to_string(),
        symbol_id: format!("repo:{repo_id}:symbol:{qualified_name}"),
        module_id: Some(format!("repo:{repo_id}:module:{module_name}")),
        name: symbol_name.to_string(),
        qualified_name,
        kind,
        path: path.to_string(),
        line_start: None,
        line_end: None,
        signature,
        audit_status: Some("unreviewed".to_string()),
        verification_state: None,
        attributes: BTreeMap::new(),
    }
}

fn upsert_symbol(symbols: &mut BTreeMap<String, SymbolRecord>, symbol: SymbolRecord) {
    match symbols.get_mut(&symbol.symbol_id) {
        Some(existing) if existing.kind == RepoSymbolKind::ModuleExport => {
            *existing = symbol;
        }
        Some(_) => {}
        None => {
            symbols.insert(symbol.symbol_id.clone(), symbol);
        }
    }
}
