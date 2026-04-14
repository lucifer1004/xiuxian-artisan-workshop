use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

use walkdir::WalkDir;
use xiuxian_wendao_core::repo_intelligence::{
    DocRecord, ExampleRecord, ImportRecord, ModuleRecord, RegisteredRepository,
    RepoIntelligenceError, SymbolRecord,
};

use super::parsing::{
    contains_documentation_annotation, parse_imports_for_repository,
    parse_safe_package_overlay_metadata, parse_safe_root_package_overlay_metadata,
    parse_symbol_declarations_for_repository,
};
use super::relations::{
    annotation_doc_title, doc_targets_for_annotation_doc, doc_targets_for_file_doc,
};
use super::types::CollectedDoc;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum RepositorySurface {
    Api,
    Example,
    Documentation,
    Support,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct NestedUsersGuideTopic {
    title: &'static str,
    format: &'static str,
}

const CONVENTIONS_SECTION_TOPICS: [NestedUsersGuideTopic; 3] = [
    NestedUsersGuideTopic {
        title: "Documentation",
        format: "modelica_users_guide_documentation",
    },
    NestedUsersGuideTopic {
        title: "ModelicaCode",
        format: "modelica_users_guide_modelica_code",
    },
    NestedUsersGuideTopic {
        title: "Icons",
        format: "modelica_users_guide_icons",
    },
];

const RELEASE_NOTES_SECTION_TOPICS: [NestedUsersGuideTopic; 1] = [NestedUsersGuideTopic {
    title: "VersionManagement",
    format: "modelica_users_guide_release_notes_version_management",
}];

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct RepositorySnapshot {
    entries: Vec<RepositoryFileEntry>,
    package_orders: BTreeMap<String, Vec<String>>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct RepositoryFileEntry {
    pub(crate) absolute_path: PathBuf,
    pub(crate) relative_path: String,
    pub(crate) surface: RepositorySurface,
    pub(crate) modelica_contents: Option<String>,
}

impl RepositorySnapshot {
    pub(crate) fn load(repository_root: &Path) -> Result<Self, RepoIntelligenceError> {
        let mut entries = Vec::new();
        let mut package_orders = BTreeMap::new();

        for absolute_path in repository_file_paths(repository_root) {
            let Some(relative_file_path) = relative_path(repository_root, absolute_path.as_path())
            else {
                continue;
            };
            let file_name = absolute_path.file_name().and_then(std::ffi::OsStr::to_str);
            let extension = absolute_path.extension().and_then(std::ffi::OsStr::to_str);
            let modelica_contents = if extension == Some("mo") {
                Some(read_repository_text_file(absolute_path.as_path())?)
            } else {
                None
            };

            if file_name == Some("package.order") {
                let parent_relative = absolute_path
                    .parent()
                    .and_then(|parent| relative_path(repository_root, parent))
                    .unwrap_or_default();
                let order_entries = read_repository_text_file(absolute_path.as_path())?
                    .lines()
                    .map(str::trim)
                    .filter(|line| !line.is_empty())
                    .filter(|line| !line.starts_with("//"))
                    .map(str::to_string)
                    .collect::<Vec<_>>();
                if !order_entries.is_empty() {
                    package_orders.insert(parent_relative, order_entries);
                }
            }

            entries.push(RepositoryFileEntry {
                surface: repository_surface(relative_file_path.as_str()),
                absolute_path,
                relative_path: relative_file_path,
                modelica_contents,
            });
        }

        Ok(Self {
            entries,
            package_orders,
        })
    }

    pub(crate) fn entries(&self) -> &[RepositoryFileEntry] {
        &self.entries
    }

    pub(crate) fn package_orders(&self) -> &BTreeMap<String, Vec<String>> {
        &self.package_orders
    }

    pub(crate) fn package_files(&self) -> Result<Vec<&RepositoryFileEntry>, RepoIntelligenceError> {
        let package_files = self
            .entries
            .iter()
            .filter(|entry| {
                entry
                    .absolute_path
                    .file_name()
                    .and_then(std::ffi::OsStr::to_str)
                    == Some("package.mo")
            })
            .collect::<Vec<_>>();
        if package_files.is_empty() {
            return Err(RepoIntelligenceError::AnalysisFailed {
                message: "no package.mo files discovered during Modelica analysis".to_string(),
            });
        }
        Ok(package_files)
    }
}

fn read_repository_text_file(path: &Path) -> Result<String, RepoIntelligenceError> {
    fs::read_to_string(path).map_err(|error| RepoIntelligenceError::AnalysisFailed {
        message: format!(
            "failed to read repository file `{}`: {error}",
            path.display()
        ),
    })
}

pub(crate) fn collect_module_records(
    repo_id: &str,
    root_package_name: &str,
    package_files: &[&RepositoryFileEntry],
    package_orders: &BTreeMap<String, Vec<String>>,
) -> Vec<ModuleRecord> {
    let mut modules = package_files
        .iter()
        .filter_map(|entry| {
            let relative = entry.relative_path.as_str();
            if relative != "package.mo" && entry.surface == RepositorySurface::Support {
                return None;
            }
            let qualified_name = qualified_module_name(root_package_name, relative)?;
            Some(ModuleRecord {
                repo_id: repo_id.to_string(),
                module_id: module_id(repo_id, qualified_name.as_str()),
                qualified_name,
                path: relative.to_string(),
            })
        })
        .collect::<Vec<_>>();
    modules.sort_by(|left, right| {
        module_sort_key(left.path.as_str(), package_orders)
            .cmp(&module_sort_key(right.path.as_str(), package_orders))
            .then_with(|| left.qualified_name.cmp(&right.qualified_name))
            .then_with(|| left.path.cmp(&right.path))
    });
    modules
}

pub(crate) fn modules_by_qualified_name(
    modules: &[ModuleRecord],
) -> BTreeMap<String, ModuleRecord> {
    modules
        .iter()
        .cloned()
        .map(|module| (module.qualified_name.clone(), module))
        .collect()
}

pub(crate) fn collect_symbol_records(
    repository: &RegisteredRepository,
    repo_id: &str,
    snapshot: &RepositorySnapshot,
    root_package_name: &str,
    modules: &BTreeMap<String, ModuleRecord>,
) -> Result<Vec<SymbolRecord>, RepoIntelligenceError> {
    let mut symbols = Vec::new();
    let mut seen = BTreeSet::new();

    for entry in snapshot.entries() {
        let Some(contents) = entry.modelica_contents.as_deref() else {
            continue;
        };
        if entry.surface != RepositorySurface::Api {
            continue;
        }
        let Some(module_qualified_name) =
            containing_module_name(root_package_name, entry.relative_path.as_str())
        else {
            continue;
        };
        let module_id = modules
            .get(module_qualified_name.as_str())
            .map(|module| module.module_id.clone());
        if safe_package_overlay_metadata_for_relative_path(
            entry.relative_path.as_str(),
            contents,
            root_package_name,
        )
        .is_some()
        {
            continue;
        }

        for declaration in parse_symbol_declarations_for_repository(
            repository,
            entry.relative_path.as_str(),
            contents,
        )? {
            let qualified_name = format!("{module_qualified_name}.{}", declaration.name);
            let symbol_id = format!("repo:{repo_id}:symbol:{qualified_name}");
            if !seen.insert(symbol_id.clone()) {
                continue;
            }
            symbols.push(SymbolRecord {
                repo_id: repo_id.to_string(),
                symbol_id,
                module_id: module_id.clone(),
                name: declaration.name,
                qualified_name,
                kind: declaration.kind,
                path: entry.relative_path.clone(),
                line_start: declaration.line_start,
                line_end: declaration.line_end,
                signature: Some(declaration.signature),
                audit_status: None,
                verification_state: None,
                attributes: declaration.attributes,
            });
        }
    }

    symbols.sort_by(|left, right| left.qualified_name.cmp(&right.qualified_name));
    Ok(symbols)
}

pub(crate) fn collect_import_records(
    repository: &RegisteredRepository,
    repo_id: &str,
    snapshot: &RepositorySnapshot,
    root_package_name: &str,
    modules: &BTreeMap<String, ModuleRecord>,
) -> Result<Vec<ImportRecord>, RepoIntelligenceError> {
    let mut imports = Vec::new();

    for entry in snapshot.entries() {
        let Some(contents) = entry.modelica_contents.as_deref() else {
            continue;
        };
        if entry.surface == RepositorySurface::Support {
            continue;
        }
        imports.extend(collect_import_records_for_file(
            repository,
            repo_id,
            entry.relative_path.as_str(),
            entry.relative_path.as_str(),
            contents,
            root_package_name,
            modules,
        )?);
    }

    imports.sort_by(|left, right| {
        left.path
            .cmp(&right.path)
            .then_with(|| left.source_module.cmp(&right.source_module))
            .then_with(|| left.import_name.cmp(&right.import_name))
            .then_with(|| left.target_package.cmp(&right.target_package))
    });
    Ok(imports)
}

pub(crate) fn collect_import_records_for_file(
    repository: &RegisteredRepository,
    repo_id: &str,
    relative_within_root: &str,
    record_path: &str,
    contents: &str,
    root_package_name: &str,
    modules: &BTreeMap<String, ModuleRecord>,
) -> Result<Vec<ImportRecord>, RepoIntelligenceError> {
    let Some(module_qualified_name) =
        containing_module_name(root_package_name, relative_within_root)
    else {
        return Ok(Vec::new());
    };
    let source_module_id = modules.get(module_qualified_name.as_str()).map_or_else(
        || module_id(repo_id, module_qualified_name.as_str()),
        |module| module.module_id.clone(),
    );
    let mut imports = Vec::new();
    let mut seen = BTreeSet::new();

    let parsed_imports = if is_api_surface_path(relative_within_root) {
        if let Some(metadata) = safe_package_overlay_metadata_for_relative_path(
            relative_within_root,
            contents,
            root_package_name,
        ) {
            metadata.imports
        } else {
            parse_imports_for_repository(repository, relative_within_root, contents)?
        }
    } else {
        parse_imports_for_repository(repository, relative_within_root, contents)?
    };

    for parsed_import in parsed_imports {
        let source_module = parsed_import.name.clone();
        let import_name = parsed_import
            .alias
            .clone()
            .unwrap_or_else(|| import_leaf_name(source_module.as_str()));
        let target_package = source_module
            .split('.')
            .next()
            .unwrap_or(source_module.as_str())
            .to_string();
        let resolved_id = modules
            .get(source_module.as_str())
            .map(|module| module.module_id.clone());
        let kind_key = match parsed_import.kind {
            xiuxian_wendao_core::repo_intelligence::ImportKind::Symbol => "symbol",
            xiuxian_wendao_core::repo_intelligence::ImportKind::Module => "module",
            xiuxian_wendao_core::repo_intelligence::ImportKind::Reexport => "reexport",
        };
        let import_key = (
            record_path.to_string(),
            source_module.clone(),
            import_name.clone(),
            kind_key,
        );
        if !seen.insert(import_key) {
            continue;
        }
        imports.push(ImportRecord {
            repo_id: repo_id.to_string(),
            module_id: source_module_id.clone(),
            path: record_path.to_string(),
            import_name,
            target_package,
            source_module,
            kind: parsed_import.kind,
            line_start: parsed_import.line_start,
            resolved_id,
            attributes: parsed_import.attributes,
        });
    }

    imports.sort_by(|left, right| {
        left.path
            .cmp(&right.path)
            .then_with(|| left.source_module.cmp(&right.source_module))
            .then_with(|| left.import_name.cmp(&right.import_name))
            .then_with(|| left.target_package.cmp(&right.target_package))
    });
    Ok(imports)
}

fn import_leaf_name(import_path: &str) -> String {
    import_path
        .rsplit('.')
        .next()
        .unwrap_or(import_path)
        .trim()
        .to_string()
}

pub(crate) fn collect_example_records(
    repo_id: &str,
    snapshot: &RepositorySnapshot,
    package_orders: &BTreeMap<String, Vec<String>>,
) -> Vec<ExampleRecord> {
    let mut examples = Vec::new();
    for entry in snapshot.entries() {
        if entry.modelica_contents.is_none() {
            continue;
        }
        if entry.surface != RepositorySurface::Example {
            continue;
        }
        if entry
            .absolute_path
            .file_name()
            .and_then(std::ffi::OsStr::to_str)
            == Some("package.mo")
        {
            continue;
        }
        let title = entry
            .absolute_path
            .file_stem()
            .and_then(std::ffi::OsStr::to_str)
            .unwrap_or("example")
            .to_string();
        examples.push(ExampleRecord {
            repo_id: repo_id.to_string(),
            example_id: format!("repo:{repo_id}:example:{}", entry.relative_path),
            title,
            path: entry.relative_path.clone(),
            summary: None,
        });
    }
    examples.sort_by(|left, right| {
        example_sort_key(left.path.as_str(), package_orders)
            .cmp(&example_sort_key(right.path.as_str(), package_orders))
            .then_with(|| left.title.cmp(&right.title))
            .then_with(|| left.path.cmp(&right.path))
    });
    examples
}

fn repository_surface(relative_path: &str) -> RepositorySurface {
    let components = path_components(relative_path);
    if components.contains(&"UsersGuide") {
        return RepositorySurface::Documentation;
    }
    if components.contains(&"Internal") {
        return RepositorySurface::Support;
    }
    if let Some(examples_index) = components
        .iter()
        .position(|component| *component == "Examples")
    {
        if components
            .iter()
            .skip(examples_index + 1)
            .any(|component| matches!(*component, "ExampleUtilities" | "Utilities"))
        {
            return RepositorySurface::Support;
        }
        return RepositorySurface::Example;
    }
    RepositorySurface::Api
}

pub(crate) fn is_api_surface_path(relative_path: &str) -> bool {
    repository_surface(relative_path) == RepositorySurface::Api
}

pub(crate) fn modelica_doc_surface_semantic_markers(
    relative_path: &str,
    contents: &str,
) -> Vec<String> {
    let mut markers = Vec::new();
    if contains_documentation_annotation(contents) {
        markers.push("annotation.documentation".to_string());
    }

    if repository_surface(relative_path) == RepositorySurface::Documentation
        && is_supported_users_guide_doc_path(Path::new(relative_path))
    {
        let file_stem = Path::new(relative_path)
            .file_stem()
            .and_then(std::ffi::OsStr::to_str)
            .unwrap_or_default();
        if file_stem.eq_ignore_ascii_case("Conventions") {
            markers.extend(
                documented_nested_users_guide_topics(contents)
                    .into_iter()
                    .map(|topic| format!("users_guide.section.{}", topic.title)),
            );
        } else if file_stem.eq_ignore_ascii_case("ReleaseNotes") {
            markers.extend(
                documented_release_notes_topics(contents)
                    .into_iter()
                    .map(|topic| format!("users_guide.section.{}", topic.title)),
            );
        }
    }

    markers.sort();
    markers
}

fn doc_format_hint(relative_path: &str, is_annotation: bool) -> Option<String> {
    if repository_surface(relative_path) == RepositorySurface::Documentation {
        return Some(users_guide_doc_format(relative_path, is_annotation));
    }
    if is_annotation {
        return Some("modelica_annotation".to_string());
    }
    Path::new(relative_path)
        .extension()
        .and_then(std::ffi::OsStr::to_str)
        .map(str::to_string)
}

fn users_guide_doc_format(relative_path: &str, is_annotation: bool) -> String {
    let components = path_components(relative_path);
    let file_stem = Path::new(relative_path)
        .file_stem()
        .and_then(std::ffi::OsStr::to_str)
        .unwrap_or_default();
    let base = if components.contains(&"Tutorial") {
        "modelica_users_guide_tutorial"
    } else if file_stem.eq_ignore_ascii_case("Conventions") {
        "modelica_users_guide_conventions"
    } else if file_stem.eq_ignore_ascii_case("Connectors") {
        "modelica_users_guide_connectors"
    } else if file_stem.eq_ignore_ascii_case("Implementation") {
        "modelica_users_guide_implementation"
    } else if file_stem.eq_ignore_ascii_case("RevisionHistory") {
        "modelica_users_guide_revision_history"
    } else if file_stem.eq_ignore_ascii_case("VersionManagement") {
        "modelica_users_guide_version_management"
    } else if components.contains(&"Overview") || file_stem.eq_ignore_ascii_case("Overview") {
        "modelica_users_guide_overview"
    } else if components.contains(&"ReleaseNotes") || file_stem.eq_ignore_ascii_case("ReleaseNotes")
    {
        "modelica_users_guide_release_notes"
    } else if components.contains(&"References") || matches!(file_stem, "References" | "Literature")
    {
        "modelica_users_guide_reference"
    } else if file_stem.eq_ignore_ascii_case("Contact") {
        "modelica_users_guide_contact"
    } else if matches!(file_stem, "Glossar" | "Glossary") {
        "modelica_users_guide_glossary"
    } else if matches!(file_stem, "Parameters" | "Parameterization") {
        "modelica_users_guide_parameter"
    } else if file_stem.eq_ignore_ascii_case("Concept") || file_stem.ends_with("Concept") {
        "modelica_users_guide_concept"
    } else {
        "modelica_users_guide_page"
    };

    if is_annotation {
        format!("{base}_annotation")
    } else {
        base.to_string()
    }
}

fn is_supported_users_guide_doc_path(path: &Path) -> bool {
    matches!(
        path.extension().and_then(std::ffi::OsStr::to_str),
        Some("mo" | "md" | "rst" | "qmd")
    )
}

fn doc_title(path: &Path) -> String {
    if path.file_name().and_then(std::ffi::OsStr::to_str) == Some("package.mo") {
        return path
            .parent()
            .and_then(Path::file_name)
            .and_then(std::ffi::OsStr::to_str)
            .unwrap_or("package")
            .to_string();
    }

    match path.extension().and_then(std::ffi::OsStr::to_str) {
        Some("mo" | "md" | "rst" | "qmd") => path
            .file_stem()
            .and_then(std::ffi::OsStr::to_str)
            .unwrap_or("doc")
            .to_string(),
        _ => path
            .file_name()
            .and_then(std::ffi::OsStr::to_str)
            .unwrap_or("doc")
            .to_string(),
    }
}

pub(crate) fn collect_doc_records(
    repo_id: &str,
    snapshot: &RepositorySnapshot,
    root_package_name: &str,
    module_lookup: &BTreeMap<String, ModuleRecord>,
    symbols: &[SymbolRecord],
    package_orders: &BTreeMap<String, Vec<String>>,
) -> Vec<CollectedDoc> {
    let root_module_id = module_lookup
        .get(root_package_name)
        .map(|module| module.module_id.clone());
    let mut docs = Vec::new();
    for entry in snapshot.entries() {
        let path = entry.absolute_path.as_path();
        let relative = entry.relative_path.as_str();
        let is_readme = path
            .file_name()
            .and_then(std::ffi::OsStr::to_str)
            .is_some_and(|name| name.to_ascii_lowercase().starts_with("readme"));
        let surface = entry.surface;
        let is_users_guide_doc =
            surface == RepositorySurface::Documentation && is_supported_users_guide_doc_path(path);
        let modelica_contents = entry.modelica_contents.as_deref();
        if is_readme || is_users_guide_doc {
            let title = doc_title(path);
            let format = doc_format_hint(relative, false);
            let target_ids = doc_targets_for_file_doc(
                relative,
                root_package_name,
                module_lookup,
                root_module_id.as_deref(),
            );
            docs.push(CollectedDoc {
                record: DocRecord {
                    repo_id: repo_id.to_string(),
                    doc_id: format!("repo:{repo_id}:doc:{relative}"),
                    title,
                    path: relative.to_string(),
                    format,
                    doc_target: None,
                },
                target_ids: target_ids.clone(),
            });
            docs.extend(collect_nested_users_guide_section_docs(
                repo_id,
                relative,
                modelica_contents,
                &target_ids,
            ));
        }

        let Some(contents) = modelica_contents else {
            continue;
        };
        if !contains_documentation_annotation(contents) {
            continue;
        }
        let target_ids = doc_targets_for_annotation_doc(
            relative,
            root_package_name,
            module_lookup,
            symbols,
            root_module_id.as_deref(),
        );
        if target_ids.is_empty() {
            continue;
        }
        docs.push(CollectedDoc {
            record: DocRecord {
                repo_id: repo_id.to_string(),
                doc_id: format!("repo:{repo_id}:doc:{relative}#annotation.documentation"),
                title: annotation_doc_title(relative, symbols),
                path: format!("{relative}#annotation.documentation"),
                format: doc_format_hint(relative, true),
                doc_target: None,
            },
            target_ids,
        });
    }
    docs.sort_by(|left, right| {
        doc_sort_key(left.record.path.as_str(), package_orders)
            .cmp(&doc_sort_key(right.record.path.as_str(), package_orders))
            .then_with(|| left.record.path.cmp(&right.record.path))
    });
    docs
}

pub(crate) fn relative_path(repository_root: &Path, path: &Path) -> Option<String> {
    path.strip_prefix(repository_root)
        .ok()
        .map(|relative| relative.to_string_lossy().replace('\\', "/"))
}

pub(crate) fn package_overlay_expected_name(
    root_package_name: &str,
    relative_package_path: &str,
) -> Option<String> {
    if relative_package_path == "package.mo" {
        return Some(root_package_name.to_string());
    }
    Path::new(relative_package_path)
        .parent()?
        .file_name()
        .and_then(std::ffi::OsStr::to_str)
        .map(str::to_string)
}

pub(crate) fn safe_package_overlay_metadata_for_relative_path(
    relative_package_path: &str,
    contents: &str,
    root_package_name: &str,
) -> Option<super::parsing::PackageOverlayMetadata> {
    if !relative_package_path.ends_with("package.mo") {
        return None;
    }
    let expected_package_name =
        package_overlay_expected_name(root_package_name, relative_package_path)?;
    parse_safe_package_overlay_metadata(contents, expected_package_name.as_str()).or_else(|| {
        if relative_package_path == "package.mo" {
            parse_safe_root_package_overlay_metadata(contents)
        } else {
            None
        }
    })
}

pub(crate) fn qualified_module_name(
    root_package_name: &str,
    relative_package_path: &str,
) -> Option<String> {
    if relative_package_path == "package.mo" {
        return Some(root_package_name.to_string());
    }
    let mut qualified = root_package_name.to_string();
    let relative_dir = Path::new(relative_package_path).parent()?;
    for component in relative_dir.components() {
        let std::path::Component::Normal(part) = component else {
            continue;
        };
        qualified.push('.');
        qualified.push_str(part.to_str()?);
    }
    Some(qualified)
}

pub(crate) fn containing_module_name(
    root_package_name: &str,
    relative_path: &str,
) -> Option<String> {
    let parent = Path::new(relative_path).parent()?;
    if parent.as_os_str().is_empty() {
        return Some(root_package_name.to_string());
    }
    let mut qualified = root_package_name.to_string();
    for component in parent.components() {
        let std::path::Component::Normal(part) = component else {
            continue;
        };
        qualified.push('.');
        qualified.push_str(part.to_str()?);
    }
    Some(qualified)
}

pub(crate) fn path_components(path: &str) -> Vec<&str> {
    path.split('/')
        .filter(|component| !component.is_empty())
        .collect()
}

fn module_id(repo_id: &str, qualified_name: &str) -> String {
    format!("repo:{repo_id}:module:{qualified_name}")
}

pub(crate) fn module_sort_key(
    path: &str,
    package_orders: &BTreeMap<String, Vec<String>>,
) -> Vec<(usize, String)> {
    if path == "package.mo" {
        return vec![(0, String::new())];
    }

    let components = path_components(path);
    let mut key = vec![(0, String::new())];
    let mut parent = String::new();
    for component in components.iter().take(components.len().saturating_sub(1)) {
        let order_index = package_orders
            .get(parent.as_str())
            .and_then(|entries| entries.iter().position(|entry| entry == component))
            .unwrap_or(usize::MAX);
        key.push((order_index, (*component).to_string()));
        if parent.is_empty() {
            parent.push_str(component);
        } else {
            parent.push('/');
            parent.push_str(component);
        }
    }
    key
}

pub(crate) fn example_sort_key(
    path: &str,
    package_orders: &BTreeMap<String, Vec<String>>,
) -> Vec<(usize, String)> {
    let components = path_components(path);
    let mut key = vec![(0, String::new())];
    let mut parent = String::new();

    for component in components.iter().take(components.len().saturating_sub(1)) {
        let order_index = package_orders
            .get(parent.as_str())
            .and_then(|entries| entries.iter().position(|entry| entry == component))
            .unwrap_or(usize::MAX);
        key.push((order_index, (*component).to_string()));
        if parent.is_empty() {
            parent.push_str(component);
        } else {
            parent.push('/');
            parent.push_str(component);
        }
    }

    let example_name = Path::new(path)
        .file_stem()
        .and_then(std::ffi::OsStr::to_str)
        .unwrap_or(path);
    let order_index = package_orders
        .get(parent.as_str())
        .and_then(|entries| entries.iter().position(|entry| entry == example_name))
        .unwrap_or(usize::MAX);
    key.push((order_index, example_name.to_string()));
    key
}

pub(crate) fn doc_sort_key(
    path: &str,
    package_orders: &BTreeMap<String, Vec<String>>,
) -> Vec<(usize, String)> {
    let (source_path, variant_rank) = match path.split_once('#') {
        Some((source_path, "annotation.documentation")) => (source_path, 2usize),
        Some((source_path, suffix)) if suffix.starts_with("section.") => (source_path, 1usize),
        Some((source_path, _)) => (source_path, 1usize),
        None => (path, 0usize),
    };
    let components = path_components(source_path);
    let mut key = vec![(0, String::new())];
    let mut parent = String::new();

    for component in components.iter().take(components.len().saturating_sub(1)) {
        let order_index = package_orders
            .get(parent.as_str())
            .and_then(|entries| entries.iter().position(|entry| entry == component))
            .unwrap_or(usize::MAX);
        key.push((order_index, (*component).to_string()));
        if parent.is_empty() {
            parent.push_str(component);
        } else {
            parent.push('/');
            parent.push_str(component);
        }
    }

    let is_package = source_path.ends_with("package.mo");
    let leaf_name = doc_leaf_name(source_path);
    let leaf_order = if is_package {
        0
    } else {
        package_orders
            .get(parent.as_str())
            .and_then(|entries| entries.iter().position(|entry| entry == leaf_name.as_str()))
            .map_or(usize::MAX, |index| index.saturating_add(1))
    };
    key.push((leaf_order, leaf_name));
    key.push((variant_rank, String::new()));
    key
}

fn collect_nested_users_guide_section_docs(
    repo_id: &str,
    relative_path: &str,
    contents: Option<&str>,
    target_ids: &[String],
) -> Vec<CollectedDoc> {
    if target_ids.is_empty() {
        return Vec::new();
    }
    let Some(contents) = contents else {
        return Vec::new();
    };
    let file_stem = Path::new(relative_path)
        .file_stem()
        .and_then(std::ffi::OsStr::to_str)
        .unwrap_or_default();
    let topics = if file_stem.eq_ignore_ascii_case("Conventions") {
        documented_nested_users_guide_topics(contents)
    } else if file_stem.eq_ignore_ascii_case("ReleaseNotes") {
        documented_release_notes_topics(contents)
    } else {
        Vec::new()
    };

    topics
        .into_iter()
        .map(|topic| CollectedDoc {
            record: DocRecord {
                repo_id: repo_id.to_string(),
                doc_id: format!("repo:{repo_id}:doc:{relative_path}#section.{}", topic.title),
                title: synthetic_section_title(topic.title),
                path: format!("{relative_path}#section.{}", topic.title),
                format: Some(topic.format.to_string()),
                doc_target: None,
            },
            target_ids: target_ids.to_vec(),
        })
        .collect()
}

fn synthetic_section_title(raw_title: &str) -> String {
    if let Some(version) = raw_title.strip_prefix("Version_") {
        return format!("Version {}", version.replace('_', "."));
    }

    let mut title = String::with_capacity(raw_title.len() + 4);
    let mut previous_is_lowercase = false;
    for ch in raw_title.chars() {
        if previous_is_lowercase && ch.is_ascii_uppercase() {
            title.push(' ');
        }
        previous_is_lowercase = ch.is_ascii_lowercase();
        title.push(ch);
    }
    title
}

fn documented_nested_users_guide_topics(contents: &str) -> Vec<NestedUsersGuideTopic> {
    CONVENTIONS_SECTION_TOPICS
        .into_iter()
        .filter(|topic| contains_documented_nested_topic(contents, topic.title))
        .collect()
}

fn documented_release_notes_topics(contents: &str) -> Vec<NestedUsersGuideTopic> {
    let mut topics = RELEASE_NOTES_SECTION_TOPICS
        .into_iter()
        .filter(|topic| contains_documented_nested_topic(contents, topic.title))
        .collect::<Vec<_>>();
    topics.extend(documented_release_notes_versions(contents));
    topics
}

fn documented_release_notes_versions(contents: &str) -> Vec<NestedUsersGuideTopic> {
    release_notes_version_names(contents)
        .into_iter()
        .filter(|version_name| contains_documented_nested_topic(contents, version_name.as_str()))
        .map(|version_name| NestedUsersGuideTopic {
            title: Box::leak(version_name.into_boxed_str()),
            format: "modelica_users_guide_release_notes_version",
        })
        .collect()
}

fn release_notes_version_names(contents: &str) -> Vec<String> {
    contents
        .lines()
        .filter_map(|line| {
            let trimmed = line.trim_start();
            if !trimmed.starts_with("class Version_") {
                return None;
            }
            trimmed
                .split_whitespace()
                .nth(1)
                .map(str::trim)
                .filter(|name| name.starts_with("Version_"))
                .map(str::to_string)
        })
        .collect()
}

fn contains_documented_nested_topic(contents: &str, topic_name: &str) -> bool {
    let Some((start, kind)) = topic_declaration_start(contents, topic_name) else {
        return false;
    };
    let end_marker = format!("end {topic_name};");
    let Some(relative_end) = contents[start..].find(end_marker.as_str()) else {
        return false;
    };
    let block = &contents[start..start + relative_end + end_marker.len()];
    block.contains("annotation (Documentation(")
        || block.contains("annotation(Documentation(")
        || (kind == "record" && block.contains("Documentation(info"))
}

fn topic_declaration_start<'a>(contents: &'a str, topic_name: &'a str) -> Option<(usize, &'a str)> {
    ["package", "class", "model", "record"]
        .into_iter()
        .find_map(|kind| {
            let marker = format!("{kind} {topic_name}");
            contents.find(marker.as_str()).map(|offset| (offset, kind))
        })
}

fn doc_leaf_name(path: &str) -> String {
    if path.ends_with("package.mo") {
        return Path::new(path)
            .parent()
            .and_then(Path::file_name)
            .and_then(std::ffi::OsStr::to_str)
            .unwrap_or("package")
            .to_string();
    }
    Path::new(path)
        .file_stem()
        .and_then(std::ffi::OsStr::to_str)
        .unwrap_or(path)
        .to_string()
}

fn repository_file_paths(repository_root: &Path) -> Vec<PathBuf> {
    let mut files = WalkDir::new(repository_root)
        .into_iter()
        .filter_entry(|entry| !should_skip_walk_entry(entry))
        .filter_map(Result::ok)
        .map(walkdir::DirEntry::into_path)
        .filter(|path| path.is_file())
        .collect::<Vec<_>>();
    files.sort();
    files
}

fn should_skip_walk_entry(entry: &walkdir::DirEntry) -> bool {
    if entry.depth() == 0 {
        return false;
    }
    entry
        .file_name()
        .to_str()
        .is_some_and(|name| name.starts_with('.'))
}

#[cfg(test)]
#[path = "../../tests/unit/plugin/discovery.rs"]
mod tests;
