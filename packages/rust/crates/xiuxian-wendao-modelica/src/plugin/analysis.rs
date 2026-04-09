use std::fs;
use std::path::{Path, PathBuf};

use walkdir::WalkDir;

use xiuxian_wendao_core::repo_intelligence::{
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
    let resolved_root = resolve_modelica_root(context, repository_root)?;
    let root_package_path = resolved_root.package_root.join("package.mo");

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

    let package_files = discover_package_files(&resolved_root.package_root)?;
    let package_orders = discover_package_orders(&resolved_root.package_root)?;
    let modules = collect_module_records(
        &context.repository.id,
        &resolved_root.package_root,
        root_package_name.as_str(),
        &package_files,
        &package_orders,
    );
    let module_lookup = modules_by_qualified_name(&modules);
    let symbols = collect_symbol_records(
        &context.repository.id,
        &resolved_root.package_root,
        root_package_name.as_str(),
        &module_lookup,
    )?;
    let imports = collect_import_records(
        &context.repository.id,
        &resolved_root.package_root,
        root_package_name.as_str(),
        &module_lookup,
    )?;
    let examples = collect_example_records(
        &context.repository.id,
        &resolved_root.package_root,
        &package_orders,
    );
    let collected_docs = collect_doc_records(
        &context.repository.id,
        &resolved_root.package_root,
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

    let mut output = RepositoryAnalysisOutput {
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
    };
    prefix_output_paths(&mut output, resolved_root.path_prefix.as_deref());
    Ok(output)
}

pub(crate) fn preflight_repository(
    context: &AnalysisContext,
    repository_root: &Path,
) -> Result<(), RepoIntelligenceError> {
    resolve_modelica_root(context, repository_root).map(|_| ())
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ResolvedModelicaRoot {
    package_root: PathBuf,
    path_prefix: Option<String>,
}

fn resolve_modelica_root(
    context: &AnalysisContext,
    repository_root: &Path,
) -> Result<ResolvedModelicaRoot, RepoIntelligenceError> {
    let root_package_path = repository_root.join("package.mo");
    if root_package_path.is_file() {
        return Ok(ResolvedModelicaRoot {
            package_root: repository_root.to_path_buf(),
            path_prefix: None,
        });
    }

    let nested_candidates = nested_package_root_candidates(context, repository_root)?;
    match nested_candidates.as_slice() {
        [] => Err(RepoIntelligenceError::UnsupportedRepositoryLayout {
            repo_id: context.repository.id.clone(),
            message: "expected a Modelica repository root package.mo or a dominant top-level package directory".to_string(),
        }),
        [candidate] => Ok(ResolvedModelicaRoot {
            package_root: candidate.package_root.clone(),
            path_prefix: Some(candidate.path_prefix.clone()),
        }),
        [first, second, ..] if first.modelica_file_count > second.modelica_file_count => Ok(
            ResolvedModelicaRoot {
                package_root: first.package_root.clone(),
                path_prefix: Some(first.path_prefix.clone()),
            },
        ),
        _ => Err(RepoIntelligenceError::UnsupportedRepositoryLayout {
            repo_id: context.repository.id.clone(),
            message: "found multiple top-level Modelica package roots without a dominant package".to_string(),
        }),
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct NestedPackageCandidate {
    package_root: PathBuf,
    path_prefix: String,
    modelica_file_count: usize,
}

fn nested_package_root_candidates(
    context: &AnalysisContext,
    repository_root: &Path,
) -> Result<Vec<NestedPackageCandidate>, RepoIntelligenceError> {
    let mut candidates = fs::read_dir(repository_root)
        .map_err(|error| RepoIntelligenceError::AnalysisFailed {
            message: format!(
                "failed to enumerate Modelica repository root `{}`: {error}",
                repository_root.display()
            ),
        })?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.is_dir() && path.join("package.mo").is_file())
        .filter_map(|package_root| {
            let path_prefix = package_root.file_name()?.to_str()?.to_string();
            Some(NestedPackageCandidate {
                modelica_file_count: count_modelica_files(package_root.as_path()),
                package_root,
                path_prefix,
            })
        })
        .collect::<Vec<_>>();

    candidates.sort_by(|left, right| {
        right
            .modelica_file_count
            .cmp(&left.modelica_file_count)
            .then_with(|| left.path_prefix.cmp(&right.path_prefix))
    });

    if candidates.is_empty() {
        return Err(RepoIntelligenceError::UnsupportedRepositoryLayout {
            repo_id: context.repository.id.clone(),
            message: "expected a Modelica repository root package.mo or a dominant top-level package directory".to_string(),
        });
    }

    Ok(candidates)
}

fn count_modelica_files(root: &Path) -> usize {
    WalkDir::new(root)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|entry| entry.file_type().is_file())
        .filter(|entry| entry.path().extension().and_then(std::ffi::OsStr::to_str) == Some("mo"))
        .count()
}

fn prefix_output_paths(output: &mut RepositoryAnalysisOutput, prefix: Option<&str>) {
    let Some(prefix) = prefix.filter(|prefix| !prefix.is_empty()) else {
        return;
    };

    for module in &mut output.modules {
        module.path = prefixed_relative_path(prefix, module.path.as_str());
    }
    for symbol in &mut output.symbols {
        symbol.path = prefixed_relative_path(prefix, symbol.path.as_str());
    }
    for example in &mut output.examples {
        example.path = prefixed_relative_path(prefix, example.path.as_str());
    }
    for doc in &mut output.docs {
        doc.path = prefixed_relative_path(prefix, doc.path.as_str());
    }
    for diagnostic in &mut output.diagnostics {
        diagnostic.path = prefixed_relative_path(prefix, diagnostic.path.as_str());
    }
}

fn prefixed_relative_path(prefix: &str, path: &str) -> String {
    if path.is_empty() {
        prefix.to_string()
    } else {
        format!("{prefix}/{path}")
    }
}

#[cfg(test)]
#[path = "../../tests/unit/plugin/analysis.rs"]
mod tests;
