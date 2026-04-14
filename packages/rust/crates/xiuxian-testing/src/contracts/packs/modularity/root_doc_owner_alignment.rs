//! Root-doc owner alignment checks for LLM-friendly Rust module layouts.

use std::collections::BTreeSet;
use std::path::Path;

use proc_macro2::{LineColumn, Span};
use syn::spanned::Spanned;
use syn::{AttrStyle, Expr, ExprLit, File, Item, ItemUse, Lit, Meta, UseTree, Visibility};

const MIN_CHILD_MODULES: usize = 3;

/// Result of evaluating whether a root doc owner hint aligns with visible exports.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum RootDocOwnerAlignmentCheck {
    /// The file is not a root-seam candidate for this rule.
    NotApplicable,
    /// The root doc owner hint and visible entry seam agree.
    Aligned,
    /// The root doc names owners that do not appear in the visible entry seam.
    Misaligned(RootDocOwnerAlignmentMetrics),
}

/// Metrics for one misaligned root-doc owner hint.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RootDocOwnerAlignmentMetrics {
    /// Root-doc line number.
    pub(crate) doc_line_number: usize,
    /// Child modules named in the root doc.
    pub(crate) named_modules: String,
    /// Child modules contributing visible entries.
    pub(crate) visible_modules: String,
}

/// Check whether a root doc owner hint matches the visible entry seam.
#[must_use]
pub(crate) fn check_root_doc_owner_alignment(
    path: &Path,
    text: &str,
) -> RootDocOwnerAlignmentCheck {
    if !is_root_seam_candidate(path) {
        return RootDocOwnerAlignmentCheck::NotApplicable;
    }

    let Ok(file) = syn::parse_file(text) else {
        return RootDocOwnerAlignmentCheck::NotApplicable;
    };

    let child_modules = collect_child_modules(&file);
    if child_modules.len() < MIN_CHILD_MODULES {
        return RootDocOwnerAlignmentCheck::Aligned;
    }

    let Some(root_doc) = collect_root_doc(&file) else {
        return RootDocOwnerAlignmentCheck::NotApplicable;
    };

    let named_modules = child_modules_named_in_doc(&root_doc.text, &child_modules);
    if named_modules.is_empty() {
        return RootDocOwnerAlignmentCheck::NotApplicable;
    }

    let visible_modules = collect_visible_source_modules(&file, &child_modules);
    if visible_modules.is_empty() {
        return RootDocOwnerAlignmentCheck::NotApplicable;
    }

    if named_modules
        .iter()
        .any(|module| visible_modules.contains(module))
    {
        return RootDocOwnerAlignmentCheck::Aligned;
    }

    RootDocOwnerAlignmentCheck::Misaligned(RootDocOwnerAlignmentMetrics {
        doc_line_number: root_doc.line_number,
        named_modules: render_modules(&named_modules),
        visible_modules: render_modules(&visible_modules),
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

fn collect_root_doc(file: &File) -> Option<RootDoc> {
    let doc_attrs = file
        .attrs
        .iter()
        .filter(|attr| matches!(attr.style, AttrStyle::Inner(_)) && attr.path().is_ident("doc"))
        .collect::<Vec<_>>();
    let first_attr = doc_attrs.first()?;
    let line_number = span_start(first_attr.span()).line;
    let text = doc_attrs
        .into_iter()
        .filter_map(extract_doc_text)
        .collect::<Vec<_>>()
        .join(" ");
    if text.trim().is_empty() {
        return None;
    }
    Some(RootDoc { line_number, text })
}

fn child_modules_named_in_doc(text: &str, child_modules: &BTreeSet<String>) -> BTreeSet<String> {
    let doc_text = text.to_lowercase();
    child_modules
        .iter()
        .filter(|module_name| doc_text.contains(&module_name.to_lowercase()))
        .cloned()
        .collect()
}

fn collect_visible_source_modules(
    file: &File,
    child_modules: &BTreeSet<String>,
) -> BTreeSet<String> {
    let mut visible_modules = BTreeSet::new();

    for item in &file.items {
        let Item::Use(item_use) = item else {
            continue;
        };
        if matches!(item_use.vis, Visibility::Inherited) {
            continue;
        }

        let reexports = collect_reexports(item_use);
        visible_modules.extend(
            reexports
                .into_iter()
                .filter_map(|segments| child_source_module(&segments, child_modules)),
        );
    }

    visible_modules
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

fn render_modules(modules: &BTreeSet<String>) -> String {
    modules.iter().cloned().collect::<Vec<_>>().join(", ")
}

fn span_start(span: Span) -> SourceLocation {
    let LineColumn { line, .. } = span.start();
    SourceLocation { line: line.max(1) }
}

#[derive(Debug, Clone)]
struct RootDoc {
    line_number: usize,
    text: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct SourceLocation {
    line: usize,
}
