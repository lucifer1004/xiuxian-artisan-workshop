//! Root-owner convergence checks for LLM-friendly Rust module layouts.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use proc_macro2::{LineColumn, Span};
use syn::spanned::Spanned;
use syn::{AttrStyle, Expr, ExprLit, File, Item, ItemUse, Lit, Meta, UseTree, Visibility};

const MIN_CHILD_MODULES: usize = 3;

/// Result of evaluating whether doc-guided and visible owner hints converge.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum RootOwnerConvergenceCheck {
    /// The file is not a root-seam candidate for this rule.
    NotApplicable,
    /// The root doc and dominant visible owner converge.
    Converged,
    /// The dominant visible owner drifted away from the doc-named owner.
    Drifted(RootOwnerConvergenceMetrics),
}

/// Metrics for one drifted root-owner seam.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RootOwnerConvergenceMetrics {
    /// Root-doc line number.
    pub(crate) doc_line_number: usize,
    /// Child modules named in the root doc.
    pub(crate) named_modules: String,
    /// Dominant visible owner module.
    pub(crate) dominant_module: String,
    /// Number of visible entries contributed by the dominant owner.
    pub(crate) dominant_count: usize,
    /// 1-based line number of the first visible export.
    pub(crate) first_export_line: usize,
}

/// Check whether the root doc owner hint and dominant visible owner converge.
#[must_use]
pub(crate) fn check_root_owner_convergence(path: &Path, text: &str) -> RootOwnerConvergenceCheck {
    if !is_root_seam_candidate(path) {
        return RootOwnerConvergenceCheck::NotApplicable;
    }

    let Ok(file) = syn::parse_file(text) else {
        return RootOwnerConvergenceCheck::NotApplicable;
    };

    let child_modules = collect_child_modules(&file);
    if child_modules.len() < MIN_CHILD_MODULES {
        return RootOwnerConvergenceCheck::Converged;
    }

    let Some(root_doc) = collect_root_doc(&file) else {
        return RootOwnerConvergenceCheck::NotApplicable;
    };
    let named_modules = child_modules_named_in_doc(&root_doc.text, &child_modules);
    if named_modules.is_empty() {
        return RootOwnerConvergenceCheck::NotApplicable;
    }

    let Some(visible_surface) = collect_visible_surface(&file, &child_modules) else {
        return RootOwnerConvergenceCheck::NotApplicable;
    };
    if visible_surface.source_counts.len() < 2 {
        return RootOwnerConvergenceCheck::Converged;
    }

    if named_modules
        .iter()
        .all(|module_name| !visible_surface.source_counts.contains_key(module_name))
    {
        return RootOwnerConvergenceCheck::NotApplicable;
    }

    let Some((dominant_module, dominant_count, second_count)) =
        dominant_source_module(&visible_surface.source_counts)
    else {
        return RootOwnerConvergenceCheck::Converged;
    };
    if dominant_count <= second_count {
        return RootOwnerConvergenceCheck::Converged;
    }

    if named_modules.contains(&dominant_module) {
        return RootOwnerConvergenceCheck::Converged;
    }

    RootOwnerConvergenceCheck::Drifted(RootOwnerConvergenceMetrics {
        doc_line_number: root_doc.line_number,
        named_modules: render_modules(&named_modules),
        dominant_module,
        dominant_count,
        first_export_line: visible_surface.first_export_line,
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

fn collect_visible_surface(
    file: &File,
    child_modules: &BTreeSet<String>,
) -> Option<VisibleSurface> {
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
            .filter_map(|segments| child_source_module(&segments, child_modules))
            .collect::<Vec<_>>();
        if matched_sources.is_empty() {
            continue;
        }

        first_export_line.get_or_insert_with(|| span_start(item_use.span()).line);
        for source_module in matched_sources {
            *source_counts.entry(source_module).or_insert(0usize) += 1;
        }
    }

    Some(VisibleSurface {
        source_counts,
        first_export_line: first_export_line?,
    })
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

fn dominant_source_module(
    source_counts: &BTreeMap<String, usize>,
) -> Option<(String, usize, usize)> {
    let mut ranked = source_counts
        .iter()
        .map(|(module_name, count)| (module_name.clone(), *count))
        .collect::<Vec<_>>();
    ranked.sort_unstable_by(|left, right| right.1.cmp(&left.1).then_with(|| left.0.cmp(&right.0)));
    let (dominant_module, dominant_count) = ranked.first()?.clone();
    let second_count = ranked.get(1).map_or(0, |(_, count)| *count);
    Some((dominant_module, dominant_count, second_count))
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

#[derive(Debug, Clone)]
struct VisibleSurface {
    source_counts: BTreeMap<String, usize>,
    first_export_line: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct SourceLocation {
    line: usize,
}
