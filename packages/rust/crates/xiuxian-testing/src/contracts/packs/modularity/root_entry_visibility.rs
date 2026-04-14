//! Root-entry visibility checks for LLM-friendly Rust module layouts.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use proc_macro2::{LineColumn, Span};
use syn::spanned::Spanned;
use syn::{File, Item, ItemMod, ItemUse, UseTree, Visibility};

/// Result of evaluating whether a root seam overexposes child entry visibility.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum RootEntryVisibilityCheck {
    /// The file is not a root-seam candidate or the parent declaration is unknown.
    NotApplicable,
    /// The root seam visibility is aligned with its parent module visibility.
    Allowed,
    /// The root seam is internal but still uses a plain `pub` entry re-export.
    Overexposed(RootEntryVisibilityMetrics),
}

/// Metrics for one overexposed root entry re-export.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RootEntryVisibilityMetrics {
    /// Rendered parent module declaration visibility.
    pub(crate) parent_visibility: String,
    /// Re-exported symbol name.
    pub(crate) symbol_name: String,
    /// 1-based line number of the plain `pub use`.
    pub(crate) line_number: usize,
    /// Rendered root-seam re-export path.
    pub(crate) rendered_path: String,
}

/// Check whether an internal root seam uses broader entry visibility than its parent.
#[must_use]
pub(crate) fn check_root_entry_visibility(
    path: &Path,
    text: &str,
    file_texts: &BTreeMap<PathBuf, String>,
) -> RootEntryVisibilityCheck {
    if !is_root_seam_candidate(path) {
        return RootEntryVisibilityCheck::NotApplicable;
    }

    let Some(module_name) = root_module_name(path) else {
        return RootEntryVisibilityCheck::NotApplicable;
    };

    let Some(parent_visibility) = resolve_parent_visibility(path, &module_name, file_texts) else {
        return RootEntryVisibilityCheck::NotApplicable;
    };
    if matches!(parent_visibility, ParentModuleVisibility::Public) {
        return RootEntryVisibilityCheck::Allowed;
    }

    let Ok(file) = syn::parse_file(text) else {
        return RootEntryVisibilityCheck::NotApplicable;
    };
    let child_modules = collect_child_modules(&file);
    if child_modules.is_empty() {
        return RootEntryVisibilityCheck::Allowed;
    }

    for item in &file.items {
        let Item::Use(item_use) = item else {
            continue;
        };
        if !matches!(item_use.vis, Visibility::Public(_)) {
            continue;
        }

        if let Some(overexposed) = overexposed_entry(item_use, &child_modules) {
            return RootEntryVisibilityCheck::Overexposed(RootEntryVisibilityMetrics {
                parent_visibility: parent_visibility.rendered_declaration(&module_name),
                symbol_name: overexposed.symbol_name,
                line_number: span_start(item_use.span()).line,
                rendered_path: overexposed.rendered_path,
            });
        }
    }

    RootEntryVisibilityCheck::Allowed
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

fn overexposed_entry(
    item_use: &ItemUse,
    child_modules: &BTreeSet<String>,
) -> Option<OverexposedEntry> {
    collect_reexports(&item_use.tree)
        .into_iter()
        .find_map(|entry| {
            child_source_module(&entry.path_segments, child_modules).map(|_| OverexposedEntry {
                symbol_name: entry.symbol_name,
                rendered_path: format!("pub use {};", render_use_tree(&item_use.tree)),
            })
        })
}

fn collect_reexports(tree: &UseTree) -> Vec<CollectedEntry> {
    let mut exports = Vec::new();
    collect_use_tree_segments(tree, &mut Vec::new(), &mut exports);
    exports
}

fn collect_use_tree_segments(
    tree: &UseTree,
    prefix: &mut Vec<String>,
    exports: &mut Vec<CollectedEntry>,
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
            exports.push(CollectedEntry {
                path_segments: segments,
                symbol_name: name.ident.to_string(),
            });
        }
        UseTree::Rename(rename) => {
            let mut segments = prefix.clone();
            segments.push(rename.ident.to_string());
            exports.push(CollectedEntry {
                path_segments: segments,
                symbol_name: rename.rename.to_string(),
            });
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

fn render_use_tree(tree: &UseTree) -> String {
    match tree {
        UseTree::Path(path) => format!("{}::{}", path.ident, render_use_tree(&path.tree)),
        UseTree::Name(name) => name.ident.to_string(),
        UseTree::Rename(rename) => format!("{} as {}", rename.ident, rename.rename),
        UseTree::Group(group) => {
            let inner = group
                .items
                .iter()
                .map(render_use_tree)
                .collect::<Vec<_>>()
                .join(", ");
            format!("{{{inner}}}")
        }
        UseTree::Glob(_) => "*".to_string(),
    }
}

fn span_start(span: Span) -> SourceLocation {
    let LineColumn { line, .. } = span.start();
    SourceLocation { line: line.max(1) }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct CollectedEntry {
    path_segments: Vec<String>,
    symbol_name: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct OverexposedEntry {
    symbol_name: String,
    rendered_path: String,
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
