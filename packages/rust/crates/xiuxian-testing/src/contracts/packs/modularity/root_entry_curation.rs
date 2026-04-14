//! Root-entry curation checks for LLM-friendly Rust module layouts.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use proc_macro2::{LineColumn, Span};
use syn::spanned::Spanned;
use syn::{File, Item, ItemMod, ItemUse, UseTree, Visibility};

const MIN_CHILD_MODULES: usize = 3;
const MIN_VISIBLE_OWNER_MODULES: usize = 2;

/// Result of evaluating whether an internal root seam keeps one visible owner.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum RootEntryCurationCheck {
    /// The file is not a root-seam candidate or the parent declaration is unknown.
    NotApplicable,
    /// The internal root seam keeps a small canonical visible owner surface.
    Curated,
    /// The internal root seam still exposes several child-owner modules.
    InternalMultiOwnerSurface(RootEntryCurationMetrics),
}

/// Metrics for one internal root seam that still exposes several owners.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RootEntryCurationMetrics {
    /// Rendered parent module declaration visibility.
    pub(crate) parent_visibility: String,
    /// Number of child-owner modules contributing visible entries.
    pub(crate) source_modules: usize,
    /// Comma-separated visible owner module names.
    pub(crate) visible_modules: String,
    /// 1-based line number of the first contributing entry export.
    pub(crate) first_export_line: usize,
}

/// Check whether an internal root seam keeps one canonical visible owner.
#[must_use]
pub(crate) fn check_root_entry_curation(
    path: &Path,
    text: &str,
    file_texts: &BTreeMap<PathBuf, String>,
) -> RootEntryCurationCheck {
    if !is_root_seam_candidate(path) {
        return RootEntryCurationCheck::NotApplicable;
    }

    let Some(module_name) = root_module_name(path) else {
        return RootEntryCurationCheck::NotApplicable;
    };

    let Some(parent_visibility) = resolve_parent_visibility(path, &module_name, file_texts) else {
        return RootEntryCurationCheck::NotApplicable;
    };
    if matches!(parent_visibility, ParentModuleVisibility::Public) {
        return RootEntryCurationCheck::Curated;
    }

    let Ok(file) = syn::parse_file(text) else {
        return RootEntryCurationCheck::NotApplicable;
    };

    let child_modules = collect_child_modules(&file);
    if child_modules.len() < MIN_CHILD_MODULES {
        return RootEntryCurationCheck::Curated;
    }

    let mut visible_modules = BTreeSet::new();
    let mut first_export_line = None;

    for item in &file.items {
        let Item::Use(item_use) = item else {
            continue;
        };
        if matches!(item_use.vis, Visibility::Inherited) {
            continue;
        }

        let matched_modules = collect_reexports(item_use)
            .into_iter()
            .filter_map(|segments| child_source_module(&segments, &child_modules))
            .collect::<BTreeSet<_>>();
        if matched_modules.is_empty() {
            continue;
        }

        first_export_line.get_or_insert_with(|| span_start(item_use.span()).line);
        visible_modules.extend(matched_modules);
    }

    if visible_modules.len() < MIN_VISIBLE_OWNER_MODULES {
        return RootEntryCurationCheck::Curated;
    }

    RootEntryCurationCheck::InternalMultiOwnerSurface(RootEntryCurationMetrics {
        parent_visibility: parent_visibility.rendered_declaration(&module_name),
        source_modules: visible_modules.len(),
        visible_modules: render_modules(&visible_modules),
        first_export_line: first_export_line.unwrap_or(1),
    })
}

fn is_root_seam_candidate(path: &Path) -> bool {
    let Some(file_name) = path.file_name().and_then(|name| name.to_str()) else {
        return false;
    };
    if matches!(file_name, "lib.rs" | "main.rs") {
        return false;
    }
    if file_name == "mod.rs" {
        return true;
    }
    if path.extension().and_then(|ext| ext.to_str()) != Some("rs") {
        return false;
    }
    path.with_extension("").is_dir()
}

fn root_module_name(path: &Path) -> Option<String> {
    if path.file_name().and_then(|name| name.to_str()) == Some("mod.rs") {
        return path
            .parent()
            .and_then(|parent| parent.file_name())
            .and_then(|name| name.to_str())
            .map(ToOwned::to_owned);
    }

    path.file_stem()
        .and_then(|stem| stem.to_str())
        .map(ToOwned::to_owned)
}

fn resolve_parent_visibility(
    path: &Path,
    module_name: &str,
    file_texts: &BTreeMap<PathBuf, String>,
) -> Option<ParentModuleVisibility> {
    for candidate in parent_candidates(path) {
        let Some(text) = file_texts.get(&candidate) else {
            continue;
        };
        let Ok(file) = syn::parse_file(text) else {
            continue;
        };
        if let Some(item_mod) = find_top_level_module(&file, module_name) {
            return Some(classify_parent_visibility(item_mod));
        }
    }
    None
}

fn parent_candidates(path: &Path) -> Vec<PathBuf> {
    let Some(parent_dir) = path.parent().map(Path::to_path_buf) else {
        return Vec::new();
    };
    let module_parent_dir = if path.file_name().and_then(|name| name.to_str()) == Some("mod.rs") {
        let Some(module_dir_parent) = parent_dir.parent() else {
            return Vec::new();
        };
        module_dir_parent.to_path_buf()
    } else {
        parent_dir
    };

    let mut candidates = Vec::new();
    for candidate in [
        module_parent_dir.join("lib.rs"),
        module_parent_dir.join("main.rs"),
        module_parent_dir.with_extension("rs"),
        module_parent_dir.join("mod.rs"),
    ] {
        if !candidates.contains(&candidate) {
            candidates.push(candidate);
        }
    }
    candidates
}

fn find_top_level_module<'a>(file: &'a File, module_name: &str) -> Option<&'a ItemMod> {
    file.items.iter().find_map(|item| match item {
        Item::Mod(item_mod) if item_mod.ident == module_name => Some(item_mod),
        _ => None,
    })
}

fn classify_parent_visibility(item_mod: &ItemMod) -> ParentModuleVisibility {
    match &item_mod.vis {
        Visibility::Inherited => ParentModuleVisibility::Internal("mod".to_string()),
        Visibility::Public(_) => ParentModuleVisibility::Public,
        Visibility::Restricted(restricted) => {
            ParentModuleVisibility::Internal(render_restricted_visibility(restricted))
        }
    }
}

fn render_restricted_visibility(restricted: &syn::VisRestricted) -> String {
    if restricted.in_token.is_some() {
        return format!("pub(in {})", render_syn_path(&restricted.path));
    }
    format!("pub({})", render_syn_path(&restricted.path))
}

fn render_syn_path(path: &syn::Path) -> String {
    path.segments
        .iter()
        .map(|segment| segment.ident.to_string())
        .collect::<Vec<_>>()
        .join("::")
}

fn collect_child_modules(file: &File) -> BTreeSet<String> {
    file.items
        .iter()
        .filter_map(|item| match item {
            Item::Mod(item_mod) if item_mod.content.is_none() => Some(item_mod.ident.to_string()),
            _ => None,
        })
        .collect()
}

fn collect_reexports(item_use: &ItemUse) -> Vec<Vec<String>> {
    let mut exports = Vec::new();
    collect_use_tree_segments(&item_use.tree, &mut Vec::new(), &mut exports);
    exports
}

fn collect_use_tree_segments(
    tree: &UseTree,
    prefix: &mut Vec<String>,
    exports: &mut Vec<Vec<String>>,
) {
    match tree {
        UseTree::Path(path) => {
            prefix.push(path.ident.to_string());
            collect_use_tree_segments(&path.tree, prefix, exports);
            prefix.pop();
        }
        UseTree::Name(name) => {
            let mut segments = prefix.clone();
            segments.push(name.ident.to_string());
            exports.push(segments);
        }
        UseTree::Rename(rename) => {
            let mut segments = prefix.clone();
            segments.push(rename.ident.to_string());
            exports.push(segments);
        }
        UseTree::Group(group) => {
            for item in &group.items {
                collect_use_tree_segments(item, prefix, exports);
            }
        }
        UseTree::Glob(_) => {}
    }
}

fn child_source_module(segments: &[String], child_modules: &BTreeSet<String>) -> Option<String> {
    segments
        .iter()
        .filter(|segment| !matches!(segment.as_str(), "self" | "crate" | "super"))
        .find(|segment| child_modules.contains(segment.as_str()))
        .cloned()
}

fn render_modules(modules: &BTreeSet<String>) -> String {
    modules.iter().cloned().collect::<Vec<_>>().join(", ")
}

fn span_start(span: Span) -> SourceLocation {
    let LineColumn { line, .. } = span.start();
    SourceLocation { line: line.max(1) }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum ParentModuleVisibility {
    Public,
    Internal(String),
}

impl ParentModuleVisibility {
    fn rendered_declaration(&self, module_name: &str) -> String {
        match self {
            Self::Public => format!("pub mod {module_name};"),
            Self::Internal(visibility) => format!("{visibility} mod {module_name};"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct SourceLocation {
    line: usize,
}
