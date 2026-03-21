use std::path::Path;

use xiuxian_wendao::analyzers::{
    AnalysisContext, DiagnosticRecord, RepoIntelligenceError, RepositoryAnalysisOutput,
    RepositoryRecord,
};

use super::discovery::{
    collect_doc_records, collect_example_records, collect_import_records, collect_module_records,
    collect_symbol_records, discover_package_files, discover_package_orders,
    modules_by_qualified_name,
};
use super::parsing::parse_package_name;
use super::relations::collect_relation_records;

pub(crate) fn analyze_repository(
    context: &AnalysisContext,
    repository_root: &Path,
) -> Result<RepositoryAnalysisOutput, RepoIntelligenceError> {
    let root_package_path = repository_root.join("package.mo");
    if !root_package_path.is_file() {
        return Err(RepoIntelligenceError::UnsupportedRepositoryLayout {
            repo_id: context.repository.id.clone(),
            message: "expected a Modelica repository root package.mo".to_string(),
        });
    }

    let root_package_contents = std::fs::read_to_string(&root_package_path).map_err(|error| {
        RepoIntelligenceError::AnalysisFailed {
            message: format!(
                "failed to read Modelica root package `{}`: {error}",
                root_package_path.display()
            ),
        }
    })?;
    let root_package_name = parse_package_name(&root_package_contents).ok_or_else(|| {
        RepoIntelligenceError::UnsupportedRepositoryLayout {
            repo_id: context.repository.id.clone(),
            message: "failed to parse root Modelica package name".to_string(),
        }
    })?;

    let package_files = discover_package_files(repository_root)?;
    let package_orders = discover_package_orders(repository_root)?;
    let modules = collect_module_records(
        &context.repository.id,
        repository_root,
        root_package_name.as_str(),
        &package_files,
        &package_orders,
    );
    let module_lookup = modules_by_qualified_name(&modules);
    let symbols = collect_symbol_records(
        &context.repository.id,
        repository_root,
        root_package_name.as_str(),
        &module_lookup,
    )?;
    let imports = collect_import_records(
        &context.repository.id,
        repository_root,
        root_package_name.as_str(),
        &module_lookup,
    )?;
    let examples =
        collect_example_records(&context.repository.id, repository_root, &package_orders)?;
    let collected_docs = collect_doc_records(
        &context.repository.id,
        repository_root,
        root_package_name.as_str(),
        &module_lookup,
        &symbols,
        &package_orders,
    )?;
    let docs = collected_docs
        .iter()
        .map(|doc| doc.record.clone())
        .collect::<Vec<_>>();
    let relations = collect_relation_records(
        &context.repository.id,
        root_package_name.as_str(),
        &modules,
        &module_lookup,
        &symbols,
        &examples,
        &collected_docs,
    );

    Ok(RepositoryAnalysisOutput {
        repository: Some(RepositoryRecord {
            repo_id: context.repository.id.clone(),
            name: root_package_name,
            path: repository_root.display().to_string(),
            url: context.repository.url.clone(),
            revision: None,
            version: None,
            uuid: None,
            dependencies: Vec::new(),
        }),
        modules,
        symbols,
        imports,
        examples,
        docs,
        relations,
        diagnostics: vec![DiagnosticRecord {
            repo_id: context.repository.id.clone(),
            path: "package.mo".to_string(),
            line: 1,
            message: "Modelica analysis is conservative and currently based on package layout plus lightweight declaration scanning.".to_string(),
            severity: "info".to_string(),
        }],
    })
}
