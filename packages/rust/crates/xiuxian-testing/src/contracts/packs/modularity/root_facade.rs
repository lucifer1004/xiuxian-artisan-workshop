//! Root-facade export heuristics for LLM-friendly Rust module layouts.

use std::collections::BTreeSet;
use std::path::Path;

use proc_macro2::{LineColumn, Span};
use syn::spanned::Spanned;
use syn::{Item, ItemUse, UseTree, Visibility};

const MIN_CHILD_MODULES: usize = 3;
const MIN_REEXPORTED_SYMBOLS: usize = 8;
const MIN_SOURCE_MODULES: usize = 3;

/// Result of evaluating whether a root facade exports too much at the top level.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum RootFacadeExportCheck {
    /// The file is not a root-facade candidate.
    NotApplicable,
    /// The root facade remains selective enough.
    CuratedFacade,
    /// The root facade forwards too many child-module symbols.
    NoisyFacade(RootFacadeExportMetrics),
}

/// Metrics describing one noisy root facade.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RootFacadeExportMetrics {
    /// Number of declared child modules.
    pub(crate) child_modules: usize,
    /// Number of re-exported symbols sourced from child modules.
    pub(crate) exported_symbols: usize,
    /// Number of child-module surfaces that contribute those exports.
    pub(crate) source_modules: usize,
    /// 1-based line number of the first contributing re-export statement.
    pub(crate) first_export_line: usize,
}

/// Check whether one Rust root facade exports too much at the top level.
#[must_use]
pub(crate) fn check_root_facade_exports(path: &Path, text: &str) -> RootFacadeExportCheck {
    if !is_root_facade_candidate(path) {
        return RootFacadeExportCheck::NotApplicable;
    }

    let Ok(file) = syn::parse_file(text) else {
        return RootFacadeExportCheck::NotApplicable;
    };

    let child_modules = file
        .items
        .iter()
        .filter_map(|item| match item {
            Item::Mod(item_mod) if item_mod.content.is_none() => Some(item_mod.ident.to_string()),
            _ => None,
        })
        .collect::<BTreeSet<_>>();

    if child_modules.len() < MIN_CHILD_MODULES {
        return RootFacadeExportCheck::CuratedFacade;
    }

    let mut exported_symbols = 0;
    let mut source_modules = BTreeSet::new();
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

        exported_symbols += matched_sources.len();
        source_modules.extend(matched_sources);
        first_export_line.get_or_insert_with(|| span_start(item_use.span()).line);
    }

    if exported_symbols < MIN_REEXPORTED_SYMBOLS || source_modules.len() < MIN_SOURCE_MODULES {
        return RootFacadeExportCheck::CuratedFacade;
    }

    RootFacadeExportCheck::NoisyFacade(RootFacadeExportMetrics {
        child_modules: child_modules.len(),
        exported_symbols,
        source_modules: source_modules.len(),
        first_export_line: first_export_line.unwrap_or(1),
    })
}

fn is_root_facade_candidate(path: &Path) -> bool {
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
        UseTree::Glob(_) => {
            let mut segments = prefix.clone();
            segments.push("*".to_string());
            exports.push(segments);
        }
    }
}

fn child_source_module(segments: &[String], child_modules: &BTreeSet<String>) -> Option<String> {
    segments
        .iter()
        .filter(|segment| !matches!(segment.as_str(), "self" | "crate" | "super"))
        .find(|segment| child_modules.contains(segment.as_str()))
        .cloned()
}

fn span_start(span: Span) -> SourceLocation {
    let LineColumn { line, .. } = span.start();
    SourceLocation { line: line.max(1) }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct SourceLocation {
    line: usize,
}
