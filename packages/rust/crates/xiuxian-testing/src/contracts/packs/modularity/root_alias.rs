//! Root-facade alias heuristics for LLM-friendly Rust module layouts.

use std::collections::BTreeSet;
use std::path::Path;

use proc_macro2::{LineColumn, Span};
use syn::spanned::Spanned;
use syn::{Item, ItemUse, UseTree, Visibility};

/// Result of evaluating whether a root facade uses public alias re-exports.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum RootFacadeAliasCheck {
    /// The file is not a root-facade candidate.
    NotApplicable,
    /// No public alias re-export was detected.
    ClearFacade,
    /// One public alias re-export from a child module was detected.
    PublicAlias(RootFacadeAlias),
}

/// One public alias re-export found in a root facade.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RootFacadeAlias {
    /// 1-based line number of the aliasing re-export.
    pub(crate) line_number: usize,
    /// Source symbol name before aliasing.
    pub(crate) source_symbol: String,
    /// Public alias name exposed at the root facade.
    pub(crate) alias_name: String,
    /// Rendered re-export path.
    pub(crate) rendered_path: String,
}

/// Check whether one root facade publicly aliases a child-module symbol.
#[must_use]
pub(crate) fn check_root_facade_aliases(path: &Path, text: &str) -> RootFacadeAliasCheck {
    if !is_root_facade_candidate(path) {
        return RootFacadeAliasCheck::NotApplicable;
    }

    let Ok(file) = syn::parse_file(text) else {
        return RootFacadeAliasCheck::NotApplicable;
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
        if !matches!(item_use.vis, Visibility::Public(_)) {
            continue;
        }

        if let Some(alias) = classify_public_alias(item_use, &child_modules) {
            return RootFacadeAliasCheck::PublicAlias(alias);
        }
    }

    RootFacadeAliasCheck::ClearFacade
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

fn classify_public_alias(
    item_use: &ItemUse,
    child_modules: &BTreeSet<String>,
) -> Option<RootFacadeAlias> {
    collect_aliases(&item_use.tree)
        .into_iter()
        .find_map(|alias| {
            child_source_module(&alias.path_segments, child_modules).map(|_| RootFacadeAlias {
                line_number: span_start(item_use.span()).line,
                source_symbol: alias.source_symbol,
                alias_name: alias.alias_name,
                rendered_path: render_use_tree(&item_use.tree),
            })
        })
}

fn collect_aliases(tree: &UseTree) -> Vec<CollectedAlias> {
    let mut aliases = Vec::new();
    collect_alias_paths(tree, &mut Vec::new(), &mut aliases);
    aliases
}

fn collect_alias_paths(
    tree: &UseTree,
    prefix: &mut Vec<String>,
    aliases: &mut Vec<CollectedAlias>,
) {
    match tree {
        UseTree::Path(path) => {
            prefix.push(path.ident.to_string());
            collect_alias_paths(&path.tree, prefix, aliases);
            prefix.pop();
        }
        UseTree::Rename(rename) => {
            let mut segments = prefix.clone();
            segments.push(rename.ident.to_string());
            aliases.push(CollectedAlias {
                path_segments: segments,
                source_symbol: rename.ident.to_string(),
                alias_name: rename.rename.to_string(),
            });
        }
        UseTree::Group(group) => {
            for item in &group.items {
                collect_alias_paths(item, prefix, aliases);
            }
        }
        UseTree::Name(_) | UseTree::Glob(_) => {}
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
struct CollectedAlias {
    path_segments: Vec<String>,
    source_symbol: String,
    alias_name: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct SourceLocation {
    line: usize,
}
