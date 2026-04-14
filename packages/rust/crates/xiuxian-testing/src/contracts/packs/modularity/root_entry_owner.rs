//! Root-entry owner heuristics for LLM-friendly Rust module layouts.

use std::collections::BTreeSet;
use std::path::Path;

use proc_macro2::{LineColumn, Span};
use syn::spanned::Spanned;
use syn::{Item, ItemUse, UseTree, Visibility};

const SUSPICIOUS_OWNER_MODULES: &[&str] = &[
    "detail", "details", "helper", "helpers", "internal", "private", "util", "utils",
];

/// Result of evaluating whether a root seam points entries at a suspicious owner.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum RootEntryOwnerCheck {
    /// The file is not a root-seam candidate.
    NotApplicable,
    /// Visible entries point at canonical-looking owner modules.
    CanonicalOwner,
    /// One visible entry points at a suspicious helper/detail module.
    SuspiciousOwner(RootEntryOwner),
}

/// One visible root entry sourced from a suspicious helper/detail module.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RootEntryOwner {
    /// 1-based line number of the re-export.
    pub(crate) line_number: usize,
    /// Child module name that owns the symbol.
    pub(crate) source_module: String,
    /// Re-exported symbol name.
    pub(crate) symbol_name: String,
    /// Rendered re-export path.
    pub(crate) rendered_path: String,
}

/// Check whether one root seam visibly exports from a helper/detail module.
#[must_use]
pub(crate) fn check_root_entry_owner(path: &Path, text: &str) -> RootEntryOwnerCheck {
    if !is_root_seam_candidate(path) {
        return RootEntryOwnerCheck::NotApplicable;
    }

    let Ok(file) = syn::parse_file(text) else {
        return RootEntryOwnerCheck::NotApplicable;
    };

    let child_modules = file
        .items
        .iter()
        .filter_map(|item| match item {
            Item::Mod(item_mod) if item_mod.content.is_none() => Some(item_mod.ident.to_string()),
            _ => None,
        })
        .collect::<BTreeSet<_>>();

    for item in &file.items {
        let Item::Use(item_use) = item else {
            continue;
        };
        if matches!(item_use.vis, Visibility::Inherited) {
            continue;
        }

        if let Some(owner) = classify_visible_entry(item_use, &child_modules) {
            return RootEntryOwnerCheck::SuspiciousOwner(owner);
        }
    }

    RootEntryOwnerCheck::CanonicalOwner
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

fn classify_visible_entry(
    item_use: &ItemUse,
    child_modules: &BTreeSet<String>,
) -> Option<RootEntryOwner> {
    collect_reexports(&item_use.tree)
        .into_iter()
        .find_map(|entry| {
            suspicious_source_module(&entry.path_segments, child_modules).map(|source_module| {
                RootEntryOwner {
                    line_number: span_start(item_use.span()).line,
                    source_module,
                    symbol_name: entry.symbol_name,
                    rendered_path: render_use_tree(&item_use.tree),
                }
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

fn suspicious_source_module(
    segments: &[String],
    child_modules: &BTreeSet<String>,
) -> Option<String> {
    segments
        .iter()
        .filter(|segment| !matches!(segment.as_str(), "self" | "crate" | "super"))
        .find(|segment| {
            child_modules.contains(segment.as_str())
                && SUSPICIOUS_OWNER_MODULES.contains(&segment.as_str())
        })
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct SourceLocation {
    line: usize,
}
