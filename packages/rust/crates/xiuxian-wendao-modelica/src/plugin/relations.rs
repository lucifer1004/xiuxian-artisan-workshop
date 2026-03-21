use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use xiuxian_wendao::analyzers::{
    ExampleRecord, ModuleRecord, RelationKind, RelationRecord, SymbolRecord,
};

use super::discovery::{containing_module_name, path_components, qualified_module_name};
use super::types::CollectedDoc;

pub(crate) fn collect_relation_records(
    repo_id: &str,
    root_package_name: &str,
    modules: &[ModuleRecord],
    module_lookup: &BTreeMap<String, ModuleRecord>,
    symbols: &[SymbolRecord],
    examples: &[ExampleRecord],
    docs: &[CollectedDoc],
) -> Vec<RelationRecord> {
    let mut relation_keys = BTreeSet::new();
    let mut relations = Vec::new();

    for module in modules {
        if module.qualified_name == root_package_name {
            continue;
        }
        if let Some((parent, _)) = module.qualified_name.rsplit_once('.') {
            if let Some(parent_module) = module_lookup.get(parent) {
                push_relation(
                    &mut relations,
                    &mut relation_keys,
                    RelationRecord {
                        repo_id: repo_id.to_string(),
                        source_id: parent_module.module_id.clone(),
                        target_id: module.module_id.clone(),
                        kind: RelationKind::Contains,
                    },
                );
            }
        }
    }

    for symbol in symbols {
        if let Some(module_id) = symbol.module_id.as_ref() {
            push_relation(
                &mut relations,
                &mut relation_keys,
                RelationRecord {
                    repo_id: repo_id.to_string(),
                    source_id: module_id.clone(),
                    target_id: symbol.symbol_id.clone(),
                    kind: RelationKind::Declares,
                },
            );
        }
    }

    let root_module_id = module_lookup
        .get(root_package_name)
        .map(|module| module.module_id.clone());

    for example in examples {
        let target_module = target_module_for_example(example.path.as_str(), root_package_name)
            .and_then(|qualified_name| module_lookup.get(qualified_name.as_str()))
            .map(|module| module.module_id.clone())
            .or_else(|| root_module_id.clone());
        if let Some(target_id) = target_module {
            push_relation(
                &mut relations,
                &mut relation_keys,
                RelationRecord {
                    repo_id: repo_id.to_string(),
                    source_id: example.example_id.clone(),
                    target_id,
                    kind: RelationKind::ExampleOf,
                },
            );
        }
    }

    for doc in docs {
        for target_id in &doc.target_ids {
            push_relation(
                &mut relations,
                &mut relation_keys,
                RelationRecord {
                    repo_id: repo_id.to_string(),
                    source_id: doc.record.doc_id.clone(),
                    target_id: target_id.clone(),
                    kind: RelationKind::Documents,
                },
            );
        }
    }

    relations
}

fn push_relation(
    relations: &mut Vec<RelationRecord>,
    relation_keys: &mut BTreeSet<String>,
    relation: RelationRecord,
) {
    let key = format!(
        "{}::{}::{}::{:?}",
        relation.repo_id, relation.source_id, relation.target_id, relation.kind
    );
    if relation_keys.insert(key) {
        relations.push(relation);
    }
}

fn target_module_for_example(example_path: &str, root_package_name: &str) -> Option<String> {
    let components = path_components(example_path);
    let examples_index = components
        .iter()
        .position(|component| *component == "Examples")?;
    if examples_index == 0 {
        return Some(root_package_name.to_string());
    }
    let mut qualified = root_package_name.to_string();
    for component in &components[..examples_index] {
        qualified.push('.');
        qualified.push_str(component);
    }
    Some(qualified)
}

pub(crate) fn doc_targets_for_file_doc(
    relative_path: &str,
    root_package_name: &str,
    module_lookup: &BTreeMap<String, ModuleRecord>,
    root_module_id: Option<&str>,
) -> Vec<String> {
    let is_readme = Path::new(relative_path)
        .file_name()
        .and_then(std::ffi::OsStr::to_str)
        .map(|name| name.to_ascii_lowercase().starts_with("readme"))
        .unwrap_or(false);
    if is_readme {
        let mut target_ids = BTreeSet::new();
        if let Some(root_module_id) = root_module_id {
            target_ids.insert(root_module_id.to_string());
        }
        return target_ids.into_iter().collect();
    }

    if is_users_guide_path(relative_path) {
        return users_guide_target_ids(
            relative_path,
            root_package_name,
            module_lookup,
            root_module_id,
        )
        .into_iter()
        .collect();
    }

    Vec::new()
}

pub(crate) fn doc_targets_for_annotation_doc(
    relative_path: &str,
    root_package_name: &str,
    module_lookup: &BTreeMap<String, ModuleRecord>,
    symbols: &[SymbolRecord],
    root_module_id: Option<&str>,
) -> Vec<String> {
    if is_users_guide_path(relative_path) {
        return users_guide_target_ids(
            relative_path,
            root_package_name,
            module_lookup,
            root_module_id,
        )
        .into_iter()
        .collect();
    }

    let mut target_ids = BTreeSet::new();
    if relative_path.ends_with("package.mo") {
        if let Some(module_qualified_name) = qualified_module_name(root_package_name, relative_path)
        {
            if let Some(module) = module_lookup.get(module_qualified_name.as_str()) {
                target_ids.insert(module.module_id.clone());
            }
        } else if let Some(root_module_id) = root_module_id {
            target_ids.insert(root_module_id.to_string());
        }
        return target_ids.into_iter().collect();
    }

    let file_stem = Path::new(relative_path)
        .file_stem()
        .and_then(std::ffi::OsStr::to_str);
    if let Some(file_stem) = file_stem {
        if let Some(symbol) = symbols
            .iter()
            .find(|symbol| symbol.path == relative_path && symbol.name == file_stem)
        {
            target_ids.insert(symbol.symbol_id.clone());
        }
    }
    if target_ids.is_empty() {
        if let Some(module_qualified_name) =
            containing_module_name(root_package_name, relative_path)
        {
            if let Some(module) = module_lookup.get(module_qualified_name.as_str()) {
                target_ids.insert(module.module_id.clone());
            }
        }
    }
    target_ids.into_iter().collect()
}

pub(crate) fn annotation_doc_title(relative_path: &str, symbols: &[SymbolRecord]) -> String {
    let source_path = relative_path
        .strip_suffix("#annotation.documentation")
        .unwrap_or(relative_path);
    if source_path.ends_with("package.mo") {
        return format!(
            "{} documentation",
            Path::new(source_path)
                .parent()
                .and_then(Path::file_name)
                .and_then(std::ffi::OsStr::to_str)
                .filter(|name| !name.is_empty())
                .unwrap_or("package")
        );
    }
    let file_stem = Path::new(source_path)
        .file_stem()
        .and_then(std::ffi::OsStr::to_str)
        .unwrap_or("symbol");
    let title = symbols
        .iter()
        .find(|symbol| symbol.path == source_path && symbol.name == file_stem)
        .map(|symbol| symbol.name.as_str())
        .unwrap_or(file_stem);
    format!("{title} documentation")
}

fn push_module_target(
    target_ids: &mut BTreeSet<String>,
    module_lookup: &BTreeMap<String, ModuleRecord>,
    module_qualified_name: &str,
) {
    if let Some(module) = module_lookup.get(module_qualified_name) {
        target_ids.insert(module.module_id.clone());
    }
}

fn users_guide_owner_module_name(relative_path: &str, root_package_name: &str) -> Option<String> {
    let components = path_components(relative_path);
    let users_guide_index = components
        .iter()
        .position(|component| *component == "UsersGuide")?;
    if users_guide_index == 0 {
        return Some(root_package_name.to_string());
    }
    let mut qualified = root_package_name.to_string();
    for component in &components[..users_guide_index] {
        qualified.push('.');
        qualified.push_str(component);
    }
    Some(qualified)
}

fn users_guide_target_ids(
    relative_path: &str,
    root_package_name: &str,
    module_lookup: &BTreeMap<String, ModuleRecord>,
    root_module_id: Option<&str>,
) -> BTreeSet<String> {
    let mut target_ids = BTreeSet::new();
    if let Some(owner_module_name) = users_guide_owner_module_name(relative_path, root_package_name)
    {
        push_module_target(&mut target_ids, module_lookup, owner_module_name.as_str());
    }
    for users_guide_module_name in
        users_guide_hierarchy_module_names(relative_path, root_package_name)
    {
        push_module_target(
            &mut target_ids,
            module_lookup,
            users_guide_module_name.as_str(),
        );
    }
    if target_ids.is_empty() {
        if let Some(root_module_id) = root_module_id {
            target_ids.insert(root_module_id.to_string());
        }
    }
    target_ids
}

fn is_users_guide_path(relative_path: &str) -> bool {
    path_components(relative_path)
        .iter()
        .any(|component| *component == "UsersGuide")
}

fn users_guide_hierarchy_module_names(relative_path: &str, root_package_name: &str) -> Vec<String> {
    let components = path_components(relative_path);
    let Some(users_guide_index) = components
        .iter()
        .position(|component| *component == "UsersGuide")
    else {
        return Vec::new();
    };
    let module_components = &components[..components.len().saturating_sub(1)];
    let mut names = Vec::new();
    for end in (users_guide_index + 1)..=module_components.len() {
        let mut qualified = root_package_name.to_string();
        for component in &module_components[..end] {
            qualified.push('.');
            qualified.push_str(component);
        }
        names.push(qualified);
    }
    names
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use serde_json::json;
    use xiuxian_wendao::analyzers::ModuleRecord;

    use super::{doc_targets_for_annotation_doc, doc_targets_for_file_doc};

    #[test]
    fn doc_targets_for_file_doc_links_users_guide_docs_to_owner_modules() {
        let module_lookup = BTreeMap::from([
            (
                "DemoLib".to_string(),
                module("repo:modelica-demo:module:DemoLib", "DemoLib", "package.mo"),
            ),
            (
                "DemoLib.UsersGuide".to_string(),
                module(
                    "repo:modelica-demo:module:DemoLib.UsersGuide",
                    "DemoLib.UsersGuide",
                    "UsersGuide/package.mo",
                ),
            ),
            (
                "DemoLib.Controllers".to_string(),
                module(
                    "repo:modelica-demo:module:DemoLib.Controllers",
                    "DemoLib.Controllers",
                    "Controllers/package.mo",
                ),
            ),
            (
                "DemoLib.Controllers.UsersGuide".to_string(),
                module(
                    "repo:modelica-demo:module:DemoLib.Controllers.UsersGuide",
                    "DemoLib.Controllers.UsersGuide",
                    "Controllers/UsersGuide/package.mo",
                ),
            ),
            (
                "DemoLib.Controllers.UsersGuide.Tutorial".to_string(),
                module(
                    "repo:modelica-demo:module:DemoLib.Controllers.UsersGuide.Tutorial",
                    "DemoLib.Controllers.UsersGuide.Tutorial",
                    "Controllers/UsersGuide/Tutorial/package.mo",
                ),
            ),
        ]);
        let payload = json!([
            {
                "path": "README.md",
                "targets": doc_targets_for_file_doc(
                    "README.md",
                    "DemoLib",
                    &module_lookup,
                    Some("repo:modelica-demo:module:DemoLib"),
                ),
            },
            {
                "path": "UsersGuide/Overview.mo",
                "targets": doc_targets_for_file_doc(
                    "UsersGuide/Overview.mo",
                    "DemoLib",
                    &module_lookup,
                    Some("repo:modelica-demo:module:DemoLib"),
                ),
            },
            {
                "path": "Controllers/UsersGuide/Tuning.mo",
                "targets": doc_targets_for_file_doc(
                    "Controllers/UsersGuide/Tuning.mo",
                    "DemoLib",
                    &module_lookup,
                    Some("repo:modelica-demo:module:DemoLib"),
                ),
            },
            {
                "path": "Controllers/UsersGuide/Tutorial/FirstSteps.mo",
                "targets": doc_targets_for_file_doc(
                    "Controllers/UsersGuide/Tutorial/FirstSteps.mo",
                    "DemoLib",
                    &module_lookup,
                    Some("repo:modelica-demo:module:DemoLib"),
                ),
            },
        ]);

        insta::assert_json_snapshot!(
            "doc_targets_for_file_doc_links_users_guide_docs_to_owner_modules",
            payload
        );
    }

    #[test]
    fn doc_targets_for_annotation_doc_links_users_guide_docs_to_owner_modules() {
        let module_lookup = BTreeMap::from([
            (
                "DemoLib".to_string(),
                module("repo:modelica-demo:module:DemoLib", "DemoLib", "package.mo"),
            ),
            (
                "DemoLib.UsersGuide".to_string(),
                module(
                    "repo:modelica-demo:module:DemoLib.UsersGuide",
                    "DemoLib.UsersGuide",
                    "UsersGuide/package.mo",
                ),
            ),
            (
                "DemoLib.Controllers".to_string(),
                module(
                    "repo:modelica-demo:module:DemoLib.Controllers",
                    "DemoLib.Controllers",
                    "Controllers/package.mo",
                ),
            ),
            (
                "DemoLib.Controllers.UsersGuide".to_string(),
                module(
                    "repo:modelica-demo:module:DemoLib.Controllers.UsersGuide",
                    "DemoLib.Controllers.UsersGuide",
                    "Controllers/UsersGuide/package.mo",
                ),
            ),
            (
                "DemoLib.Controllers.UsersGuide.Tutorial".to_string(),
                module(
                    "repo:modelica-demo:module:DemoLib.Controllers.UsersGuide.Tutorial",
                    "DemoLib.Controllers.UsersGuide.Tutorial",
                    "Controllers/UsersGuide/Tutorial/package.mo",
                ),
            ),
        ]);
        let payload = json!([
            {
                "path": "UsersGuide/Overview.mo",
                "targets": doc_targets_for_annotation_doc(
                    "UsersGuide/Overview.mo",
                    "DemoLib",
                    &module_lookup,
                    &[],
                    Some("repo:modelica-demo:module:DemoLib"),
                ),
            },
            {
                "path": "Controllers/UsersGuide/Tuning.mo",
                "targets": doc_targets_for_annotation_doc(
                    "Controllers/UsersGuide/Tuning.mo",
                    "DemoLib",
                    &module_lookup,
                    &[],
                    Some("repo:modelica-demo:module:DemoLib"),
                ),
            },
            {
                "path": "Controllers/UsersGuide/Tutorial/FirstSteps.mo",
                "targets": doc_targets_for_annotation_doc(
                    "Controllers/UsersGuide/Tutorial/FirstSteps.mo",
                    "DemoLib",
                    &module_lookup,
                    &[],
                    Some("repo:modelica-demo:module:DemoLib"),
                ),
            },
        ]);

        insta::assert_json_snapshot!(
            "doc_targets_for_annotation_doc_links_users_guide_docs_to_owner_modules",
            payload
        );
    }

    fn module(module_id: &str, qualified_name: &str, path: &str) -> ModuleRecord {
        ModuleRecord {
            repo_id: "modelica-demo".to_string(),
            module_id: module_id.to_string(),
            qualified_name: qualified_name.to_string(),
            path: path.to_string(),
        }
    }
}
