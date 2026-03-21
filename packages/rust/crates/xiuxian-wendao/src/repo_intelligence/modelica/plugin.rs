use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use walkdir::WalkDir;

use crate::repo_intelligence::checkout::discover_checkout_metadata;
use crate::repo_intelligence::config::RegisteredRepository;
use crate::repo_intelligence::errors::RepoIntelligenceError;
use crate::repo_intelligence::plugin::{
    AnalysisContext, PluginAnalysisOutput, PluginLinkContext, RepoIntelligencePlugin,
    RepoSourceFile, RepositoryAnalysisOutput,
};
use crate::repo_intelligence::records::{
    DocRecord, ExampleRecord, ModuleRecord, RelationKind, RelationRecord, RepositoryRecord,
};

/// Built-in Modelica repository analyzer for Repo Intelligence.
#[derive(Debug, Default, Clone, Copy)]
pub struct ModelicaRepoIntelligencePlugin;

impl RepoIntelligencePlugin for ModelicaRepoIntelligencePlugin {
    fn id(&self) -> &'static str {
        "modelica"
    }

    fn supports_repository(&self, repository: &RegisteredRepository) -> bool {
        repository
            .plugins
            .iter()
            .any(|plugin| plugin.id() == self.id())
    }

    fn analyze_file(
        &self,
        _context: &AnalysisContext,
        _file: &RepoSourceFile,
    ) -> Result<PluginAnalysisOutput, RepoIntelligenceError> {
        Ok(PluginAnalysisOutput::default())
    }

    fn analyze_repository(
        &self,
        context: &AnalysisContext,
        repository_root: &Path,
    ) -> Result<RepositoryAnalysisOutput, RepoIntelligenceError> {
        let package_root = resolve_package_root(context, repository_root)?;
        let package_name = package_root
            .file_name()
            .and_then(|value| value.to_str())
            .unwrap_or("Modelica")
            .to_string();

        let modules = discover_modules(
            context.repository.id.as_str(),
            repository_root,
            package_root.as_path(),
            package_name.as_str(),
        )?;
        let examples = discover_examples(context.repository.id.as_str(), repository_root, package_root.as_path())?;
        let docs = discover_docs(context.repository.id.as_str(), repository_root, package_root.as_path())?;
        let relations = build_structural_relations(
            context.repository.id.as_str(),
            package_name.as_str(),
            package_root.as_path(),
            &modules,
            &examples,
            &docs,
        )?;

        let checkout = discover_checkout_metadata(repository_root).unwrap_or_default();
        Ok(RepositoryAnalysisOutput {
            repository: Some(RepositoryRecord {
                repo_id: context.repository.id.clone(),
                name: package_name,
                path: repository_root.display().to_string(),
                url: context
                    .repository
                    .url
                    .clone()
                    .or(checkout.remote_url.clone()),
                revision: checkout.revision,
                version: None,
                uuid: None,
                dependencies: Vec::new(),
            }),
            modules,
            symbols: Vec::new(),
            examples,
            docs,
            relations,
            diagnostics: Vec::new(),
        })
    }

    fn enrich_relations(
        &self,
        _context: &PluginLinkContext,
    ) -> Result<Vec<RelationRecord>, RepoIntelligenceError> {
        Ok(Vec::new())
    }
}

fn resolve_package_root(
    context: &AnalysisContext,
    repository_root: &Path,
) -> Result<PathBuf, RepoIntelligenceError> {
    let modelica_root = repository_root.join("Modelica");
    if modelica_root.join("package.mo").is_file() {
        return Ok(modelica_root);
    }
    if repository_root.join("package.mo").is_file() {
        return Ok(repository_root.to_path_buf());
    }

    for entry in fs::read_dir(repository_root).map_err(|error| RepoIntelligenceError::AnalysisFailed {
        message: format!("failed to read `{}`: {error}", repository_root.display()),
    })? {
        let path = entry
            .map_err(|error| RepoIntelligenceError::AnalysisFailed {
                message: format!("failed to enumerate `{}`: {error}", repository_root.display()),
            })?
            .path();
        if path.is_dir() && path.join("package.mo").is_file() {
            return Ok(path);
        }
    }

    Err(RepoIntelligenceError::UnsupportedRepositoryLayout {
        repo_id: context.repository.id.clone(),
        message: "missing Modelica package root (`Modelica/package.mo` or `package.mo`)".to_string(),
    })
}

fn discover_modules(
    repo_id: &str,
    repository_root: &Path,
    package_root: &Path,
    package_name: &str,
) -> Result<Vec<ModuleRecord>, RepoIntelligenceError> {
    let mut modules = BTreeMap::new();
    for entry in WalkDir::new(package_root).into_iter().filter_map(Result::ok) {
        if !entry.file_type().is_file() || entry.file_name().to_string_lossy() != "package.mo" {
            continue;
        }
        let qualified_name = qualified_module_name_for_package_file(package_root, package_name, entry.path());
        let module_id = format!("repo:{repo_id}:module:{qualified_name}");
        let path = relative_path_string(repository_root, entry.path())?;
        modules.entry(module_id.clone()).or_insert(ModuleRecord {
            repo_id: repo_id.to_string(),
            module_id,
            qualified_name,
            path,
        });
    }
    Ok(modules.into_values().collect())
}

fn qualified_module_name_for_package_file(
    package_root: &Path,
    package_name: &str,
    package_file: &Path,
) -> String {
    let Some(parent) = package_file.parent() else {
        return package_name.to_string();
    };
    let relative = parent.strip_prefix(package_root).ok();
    let segments = relative
        .map(|path| {
            path.components()
                .filter_map(|component| component.as_os_str().to_str())
                .map(str::trim)
                .filter(|segment| !segment.is_empty())
                .map(str::to_string)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    if segments.is_empty() {
        package_name.to_string()
    } else {
        format!("{package_name}.{}", segments.join("."))
    }
}

fn discover_examples(
    repo_id: &str,
    repository_root: &Path,
    package_root: &Path,
) -> Result<Vec<ExampleRecord>, RepoIntelligenceError> {
    let mut examples = Vec::new();
    for entry in WalkDir::new(package_root).into_iter().filter_map(Result::ok) {
        if !entry.file_type().is_file() {
            continue;
        }
        if entry.path().extension().and_then(|value| value.to_str()) != Some("mo") {
            continue;
        }
        if !path_contains_component(entry.path(), "Examples") {
            continue;
        }
        let relative = relative_path_string(repository_root, entry.path())?;
        let title = entry
            .path()
            .file_stem()
            .and_then(|value| value.to_str())
            .unwrap_or("example")
            .to_string();
        examples.push(ExampleRecord {
            repo_id: repo_id.to_string(),
            example_id: format!("repo:{repo_id}:example:{relative}"),
            title,
            path: relative,
            summary: None,
        });
    }
    Ok(examples)
}

fn discover_docs(
    repo_id: &str,
    repository_root: &Path,
    package_root: &Path,
) -> Result<Vec<DocRecord>, RepoIntelligenceError> {
    let mut docs = Vec::new();
    for entry in fs::read_dir(repository_root).map_err(|error| RepoIntelligenceError::AnalysisFailed {
        message: format!("failed to read `{}`: {error}", repository_root.display()),
    })? {
        let path = entry
            .map_err(|error| RepoIntelligenceError::AnalysisFailed {
                message: format!("failed to enumerate `{}`: {error}", repository_root.display()),
            })?
            .path();
        if !path.is_file() {
            continue;
        }
        let Some(name) = path.file_name().and_then(|value| value.to_str()) else {
            continue;
        };
        if !name.to_ascii_uppercase().starts_with("README") {
            continue;
        }
        let relative = relative_path_string(repository_root, &path)?;
        docs.push(DocRecord {
            repo_id: repo_id.to_string(),
            doc_id: format!("repo:{repo_id}:doc:{relative}"),
            title: name.to_string(),
            path: relative,
            format: path
                .extension()
                .and_then(|value| value.to_str())
                .map(str::to_string),
        });
    }

    for entry in WalkDir::new(package_root).into_iter().filter_map(Result::ok) {
        if !entry.file_type().is_file() {
            continue;
        }
        let extension = entry.path().extension().and_then(|value| value.to_str());
        if extension != Some("mo") && extension != Some("md") {
            continue;
        }
        if !path_contains_component(entry.path(), "UsersGuide") {
            continue;
        }
        let relative = relative_path_string(repository_root, entry.path())?;
        let title = entry
            .path()
            .file_stem()
            .and_then(|value| value.to_str())
            .unwrap_or("doc")
            .to_string();
        docs.push(DocRecord {
            repo_id: repo_id.to_string(),
            doc_id: format!("repo:{repo_id}:doc:{relative}"),
            title,
            path: relative,
            format: extension.map(str::to_string),
        });
    }
    Ok(docs)
}

fn build_structural_relations(
    repo_id: &str,
    package_name: &str,
    package_root: &Path,
    modules: &[ModuleRecord],
    examples: &[ExampleRecord],
    docs: &[DocRecord],
) -> Result<Vec<RelationRecord>, RepoIntelligenceError> {
    let repository_node_id = format!("repo:{repo_id}");
    let mut relations = Vec::new();
    let module_lookup = modules
        .iter()
        .map(|module| (module.qualified_name.clone(), module.module_id.clone()))
        .collect::<BTreeMap<_, _>>();

    relations.extend(modules.iter().map(|module| RelationRecord {
        repo_id: repo_id.to_string(),
        source_id: repository_node_id.clone(),
        target_id: module.module_id.clone(),
        kind: RelationKind::Contains,
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

    for module in modules {
        if let Some((parent_name, _)) = module.qualified_name.rsplit_once('.')
            && let Some(parent_id) = module_lookup.get(parent_name)
        {
            relations.push(RelationRecord {
                repo_id: repo_id.to_string(),
                source_id: parent_id.clone(),
                target_id: module.module_id.clone(),
                kind: RelationKind::Contains,
            });
        }
    }

    for example in examples {
        let module_name = module_qualified_name_from_marker(
            package_name,
            package_root,
            Path::new(example.path.as_str()),
            "Examples",
        );
        if let Some(module_id) = module_lookup.get(module_name.as_str()) {
            relations.push(RelationRecord {
                repo_id: repo_id.to_string(),
                source_id: example.example_id.clone(),
                target_id: module_id.clone(),
                kind: RelationKind::ExampleOf,
            });
        }
    }

    for doc in docs {
        let doc_path = Path::new(doc.path.as_str());
        if !path_contains_component(doc_path, "UsersGuide") {
            continue;
        }
        let module_name =
            module_qualified_name_from_marker(package_name, package_root, doc_path, "UsersGuide");
        if let Some(module_id) = module_lookup.get(module_name.as_str()) {
            relations.push(RelationRecord {
                repo_id: repo_id.to_string(),
                source_id: doc.doc_id.clone(),
                target_id: module_id.clone(),
                kind: RelationKind::Documents,
            });
        }
    }

    Ok(relations)
}

fn module_qualified_name_from_marker(
    package_name: &str,
    package_root: &Path,
    path: &Path,
    marker: &str,
) -> String {
    let relative = path.strip_prefix(package_root).unwrap_or(path);
    let segments = relative
        .components()
        .filter_map(|component| component.as_os_str().to_str())
        .map(str::trim)
        .filter(|segment| !segment.is_empty())
        .map(str::to_string)
        .collect::<Vec<_>>();
    let marker_index = segments
        .iter()
        .position(|segment| segment.eq_ignore_ascii_case(marker));
    let Some(marker_index) = marker_index else {
        return package_name.to_string();
    };
    if marker_index == 0 {
        package_name.to_string()
    } else {
        let mut module_segments = segments[..marker_index].to_vec();
        if module_segments
            .first()
            .is_some_and(|segment| segment.eq_ignore_ascii_case(package_name))
        {
            module_segments.remove(0);
        }
        if module_segments.is_empty() {
            package_name.to_string()
        } else {
            format!("{package_name}.{}", module_segments.join("."))
        }
    }
}

fn path_contains_component(path: &Path, needle: &str) -> bool {
    path.components()
        .filter_map(|component| component.as_os_str().to_str())
        .any(|component| component.eq_ignore_ascii_case(needle))
}

fn relative_path_string(
    root: &Path,
    path: &Path,
) -> Result<String, RepoIntelligenceError> {
    let relative =
        path.strip_prefix(root)
            .map_err(|error| RepoIntelligenceError::AnalysisFailed {
                message: format!(
                    "failed to compute relative path for `{}` against `{}`: {error}",
                    path.display(),
                    root.display()
                ),
            })?;
    Ok(relative.to_string_lossy().replace('\\', "/"))
}
