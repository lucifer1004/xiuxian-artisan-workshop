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
    parse_symbol_declarations_for_repository,
};
use super::relations::{
    annotation_doc_title, doc_targets_for_annotation_doc, doc_targets_for_file_doc,
};
use super::types::CollectedDoc;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum RepositorySurface {
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

pub(crate) fn discover_package_files(
    repository_root: &Path,
) -> Result<Vec<PathBuf>, RepoIntelligenceError> {
    let package_files = repository_file_paths(repository_root)
        .into_iter()
        .filter(|path| path.file_name().and_then(std::ffi::OsStr::to_str) == Some("package.mo"))
        .collect::<Vec<_>>();
    if package_files.is_empty() {
        return Err(RepoIntelligenceError::AnalysisFailed {
            message: "no package.mo files discovered during Modelica analysis".to_string(),
        });
    }
    Ok(package_files)
}

pub(crate) fn discover_package_orders(
    repository_root: &Path,
) -> Result<BTreeMap<String, Vec<String>>, RepoIntelligenceError> {
    let mut orders = BTreeMap::new();
    for path in repository_file_paths(repository_root) {
        if path.file_name().and_then(std::ffi::OsStr::to_str) != Some("package.order") {
            continue;
        }
        let parent_relative = path
            .parent()
            .and_then(|parent| relative_path(repository_root, parent))
            .unwrap_or_default();
        let contents =
            fs::read_to_string(&path).map_err(|error| RepoIntelligenceError::AnalysisFailed {
                message: format!("failed to read package.order `{}`: {error}", path.display()),
            })?;
        let entries = contents
            .lines()
            .map(str::trim)
            .filter(|line| !line.is_empty())
            .filter(|line| !line.starts_with("//"))
            .map(str::to_string)
            .collect::<Vec<_>>();
        if !entries.is_empty() {
            orders.insert(parent_relative, entries);
        }
    }
    Ok(orders)
}

pub(crate) fn collect_module_records(
    repo_id: &str,
    repository_root: &Path,
    root_package_name: &str,
    package_files: &[PathBuf],
    package_orders: &BTreeMap<String, Vec<String>>,
) -> Vec<ModuleRecord> {
    let mut modules = package_files
        .iter()
        .filter_map(|path| {
            let relative = relative_path(repository_root, path)?;
            if relative != "package.mo"
                && repository_surface(relative.as_str()) == RepositorySurface::Support
            {
                return None;
            }
            let qualified_name = qualified_module_name(root_package_name, relative.as_str())?;
            Some(ModuleRecord {
                repo_id: repo_id.to_string(),
                module_id: module_id(repo_id, qualified_name.as_str()),
                qualified_name,
                path: relative,
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
    repository_root: &Path,
    root_package_name: &str,
    modules: &BTreeMap<String, ModuleRecord>,
) -> Result<Vec<SymbolRecord>, RepoIntelligenceError> {
    let mut symbols = Vec::new();
    let mut seen = BTreeSet::new();

    for path in repository_file_paths(repository_root) {
        if !path.is_file() || path.extension().and_then(std::ffi::OsStr::to_str) != Some("mo") {
            continue;
        }
        let Some(relative) = relative_path(repository_root, &path) else {
            continue;
        };
        if repository_surface(relative.as_str()) != RepositorySurface::Api {
            continue;
        }
        let Some(module_qualified_name) =
            containing_module_name(root_package_name, relative.as_str())
        else {
            continue;
        };
        let module_id = modules
            .get(module_qualified_name.as_str())
            .map(|module| module.module_id.clone());
        let contents =
            fs::read_to_string(&path).map_err(|error| RepoIntelligenceError::AnalysisFailed {
                message: format!("failed to read Modelica file `{}`: {error}", path.display()),
            })?;

        for declaration in
            parse_symbol_declarations_for_repository(repository, relative.as_str(), &contents)?
        {
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
                path: relative.clone(),
                line_start: declaration.line_start,
                line_end: declaration.line_end,
                signature: Some(declaration.signature),
                audit_status: None,
                verification_state: None,
                attributes: if declaration.equations.is_empty() {
                    BTreeMap::new()
                } else {
                    let mut attrs = BTreeMap::new();
                    attrs.insert(
                        "equation_latex".to_string(),
                        declaration.equations.join("\n\n"),
                    );
                    attrs
                },
            });
        }
    }

    symbols.sort_by(|left, right| left.qualified_name.cmp(&right.qualified_name));
    Ok(symbols)
}

pub(crate) fn collect_import_records(
    repository: &RegisteredRepository,
    repo_id: &str,
    repository_root: &Path,
    root_package_name: &str,
    modules: &BTreeMap<String, ModuleRecord>,
) -> Result<Vec<ImportRecord>, RepoIntelligenceError> {
    let mut imports = Vec::new();
    let mut seen = BTreeSet::new();

    for path in repository_file_paths(repository_root) {
        if !path.is_file() || path.extension().and_then(std::ffi::OsStr::to_str) != Some("mo") {
            continue;
        }
        let Some(relative) = relative_path(repository_root, &path) else {
            continue;
        };
        if repository_surface(relative.as_str()) == RepositorySurface::Support {
            continue;
        }
        let Some(module_qualified_name) =
            containing_module_name(root_package_name, relative.as_str())
        else {
            continue;
        };
        let source_module_id = modules.get(module_qualified_name.as_str()).map_or_else(
            || module_id(repo_id, module_qualified_name.as_str()),
            |module| module.module_id.clone(),
        );

        let contents =
            fs::read_to_string(&path).map_err(|error| RepoIntelligenceError::AnalysisFailed {
                message: format!("failed to read Modelica file `{}`: {error}", path.display()),
            })?;

        for parsed_import in parse_imports_for_repository(repository, relative.as_str(), &contents)?
        {
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
            let import_key = (source_module.clone(), import_name.clone(), kind_key);
            if !seen.insert(import_key) {
                continue;
            }
            imports.push(ImportRecord {
                repo_id: repo_id.to_string(),
                module_id: source_module_id.clone(),
                import_name,
                target_package,
                source_module,
                kind: parsed_import.kind,
                resolved_id,
            });
        }
    }

    imports.sort_by(|left, right| {
        left.source_module
            .cmp(&right.source_module)
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
    repository_root: &Path,
    package_orders: &BTreeMap<String, Vec<String>>,
) -> Vec<ExampleRecord> {
    let mut examples = Vec::new();
    for path in repository_file_paths(repository_root) {
        if !path.is_file() || path.extension().and_then(std::ffi::OsStr::to_str) != Some("mo") {
            continue;
        }
        let Some(relative) = relative_path(repository_root, &path) else {
            continue;
        };
        if repository_surface(relative.as_str()) != RepositorySurface::Example {
            continue;
        }
        if path.file_name().and_then(std::ffi::OsStr::to_str) == Some("package.mo") {
            continue;
        }
        let title = path
            .file_stem()
            .and_then(std::ffi::OsStr::to_str)
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
    repository_root: &Path,
    root_package_name: &str,
    module_lookup: &BTreeMap<String, ModuleRecord>,
    symbols: &[SymbolRecord],
    package_orders: &BTreeMap<String, Vec<String>>,
) -> Result<Vec<CollectedDoc>, RepoIntelligenceError> {
    let root_module_id = module_lookup
        .get(root_package_name)
        .map(|module| module.module_id.clone());
    let mut docs = Vec::new();
    for path in repository_file_paths(repository_root) {
        if !path.is_file() {
            continue;
        }
        let Some(relative) = relative_path(repository_root, &path) else {
            continue;
        };
        let is_readme = path
            .file_name()
            .and_then(std::ffi::OsStr::to_str)
            .is_some_and(|name| name.to_ascii_lowercase().starts_with("readme"));
        let surface = repository_surface(relative.as_str());
        let is_users_guide_doc =
            surface == RepositorySurface::Documentation && is_supported_users_guide_doc_path(&path);
        let modelica_contents = if path.extension().and_then(std::ffi::OsStr::to_str) == Some("mo")
        {
            Some(fs::read_to_string(&path).map_err(|error| {
                RepoIntelligenceError::AnalysisFailed {
                    message: format!("failed to read Modelica file `{}`: {error}", path.display()),
                }
            })?)
        } else {
            None
        };
        if is_readme || is_users_guide_doc {
            let title = doc_title(&path);
            let format = doc_format_hint(relative.as_str(), false);
            let target_ids = doc_targets_for_file_doc(
                relative.as_str(),
                root_package_name,
                module_lookup,
                root_module_id.as_deref(),
            );
            docs.push(CollectedDoc {
                record: DocRecord {
                    repo_id: repo_id.to_string(),
                    doc_id: format!("repo:{repo_id}:doc:{relative}"),
                    title,
                    path: relative.clone(),
                    format,
                },
                target_ids: target_ids.clone(),
            });
            docs.extend(collect_nested_users_guide_section_docs(
                repo_id,
                relative.as_str(),
                modelica_contents.as_deref(),
                &target_ids,
            ));
        }

        let Some(contents) = modelica_contents.as_deref() else {
            continue;
        };
        if !contains_documentation_annotation(contents) {
            continue;
        }
        let target_ids = doc_targets_for_annotation_doc(
            relative.as_str(),
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
                title: annotation_doc_title(relative.as_str(), symbols),
                path: format!("{relative}#annotation.documentation"),
                format: doc_format_hint(relative.as_str(), true),
            },
            target_ids,
        });
    }
    docs.sort_by(|left, right| {
        doc_sort_key(left.record.path.as_str(), package_orders)
            .cmp(&doc_sort_key(right.record.path.as_str(), package_orders))
            .then_with(|| left.record.path.cmp(&right.record.path))
    });
    Ok(docs)
}

pub(crate) fn relative_path(repository_root: &Path, path: &Path) -> Option<String> {
    path.strip_prefix(repository_root)
        .ok()
        .map(|relative| relative.to_string_lossy().replace('\\', "/"))
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
