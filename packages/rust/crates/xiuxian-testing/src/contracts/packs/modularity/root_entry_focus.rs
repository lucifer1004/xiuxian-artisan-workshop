//! Root-entry focus checks for LLM-friendly Rust module layouts.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use proc_macro2::{LineColumn, Span};
use syn::spanned::Spanned;
use syn::{AttrStyle, Expr, ExprLit, File, Item, ItemUse, Lit, Meta, UseTree, Visibility};

const MIN_CHILD_MODULES: usize = 3;
const MIN_SOURCE_MODULES: usize = 3;

/// Result of evaluating whether a root seam identifies a primary entry owner.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum RootEntryFocusCheck {
    /// The file is not a root-seam candidate.
    NotApplicable,
    /// The root seam already has a clear primary owner.
    FocusedEntry,
    /// The visible entry surface spans several peer owners with no primary hint.
    UnfocusedEntry(RootEntryFocusMetrics),
}

/// Metrics describing one unfocused root entry surface.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RootEntryFocusMetrics {
    /// Number of child modules contributing visible entry exports.
    pub(crate) source_modules: usize,
    /// Count of entries contributed by the strongest source module.
    pub(crate) top_source_count: usize,
    /// 1-based line number of the first contributing export.
    pub(crate) first_export_line: usize,
}

/// Check whether a root seam identifies one primary entry owner.
#[must_use]
pub(crate) fn check_root_entry_focus(path: &Path, text: &str) -> RootEntryFocusCheck {
    if !is_root_seam_candidate(path) {
        return RootEntryFocusCheck::NotApplicable;
    }

    let Ok(file) = syn::parse_file(text) else {
        return RootEntryFocusCheck::NotApplicable;
    };

    let child_modules = collect_child_modules(&file);
    if child_modules.len() < MIN_CHILD_MODULES {
        return RootEntryFocusCheck::FocusedEntry;
    }

    let mut source_counts = BTreeMap::new();
    let mut first_export_line = None;

    for item in &file.items {
        let Item::Use(item_use) = item else {
            continue;
        };
        if matches!(item_use.vis, Visibility::Inherited) {
            continue;
        }

        let reexports = collect_reexports(item_use);
        let matched_sources = reexports
            .into_iter()
            .filter_map(|segments| child_source_module(&segments, &child_modules))
            .collect::<Vec<_>>();
        if matched_sources.is_empty() {
            continue;
        }

        first_export_line.get_or_insert_with(|| span_start(item_use.span()).line);
        for source_module in matched_sources {
            *source_counts.entry(source_module).or_insert(0usize) += 1;
        }
    }

    if source_counts.len() < MIN_SOURCE_MODULES {
        return RootEntryFocusCheck::FocusedEntry;
    }

    if has_named_primary_owner(&file, &source_counts.keys().map(String::as_str).collect()) {
        return RootEntryFocusCheck::FocusedEntry;
    }

    let mut counts = source_counts.values().copied().collect::<Vec<_>>();
    counts.sort_unstable_by(|left, right| right.cmp(left));
    let Some(top_source_count) = counts.first().copied() else {
        return RootEntryFocusCheck::FocusedEntry;
    };
    let second_source_count = counts.get(1).copied().unwrap_or(0);
    if top_source_count > second_source_count {
        return RootEntryFocusCheck::FocusedEntry;
    }

    RootEntryFocusCheck::UnfocusedEntry(RootEntryFocusMetrics {
        source_modules: source_counts.len(),
        top_source_count,
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

fn has_named_primary_owner(file: &File, source_modules: &BTreeSet<&str>) -> bool {
    let doc_text = collect_root_doc_text(file);
    let Some(doc_text) = doc_text else {
        return false;
    };
    let doc_text = doc_text.to_lowercase();
    source_modules
        .iter()
        .any(|module_name| doc_text.contains(&module_name.to_lowercase()))
}

fn collect_root_doc_text(file: &File) -> Option<String> {
    let text = file
        .attrs
        .iter()
        .filter(|attr| matches!(attr.style, AttrStyle::Inner(_)) && attr.path().is_ident("doc"))
        .filter_map(extract_doc_text)
        .collect::<Vec<_>>()
        .join(" ");
    if text.trim().is_empty() {
        None
    } else {
        Some(text)
    }
}

fn extract_doc_text(attr: &syn::Attribute) -> Option<String> {
    let Meta::NameValue(name_value) = &attr.meta else {
        return None;
    };
    let Expr::Lit(ExprLit {
        lit: Lit::Str(lit_str),
        ..
    }) = &name_value.value
    else {
        return None;
    };
    Some(lit_str.value())
}

fn span_start(span: Span) -> SourceLocation {
    let LineColumn { line, .. } = span.start();
    SourceLocation { line: line.max(1) }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct SourceLocation {
    line: usize,
}
