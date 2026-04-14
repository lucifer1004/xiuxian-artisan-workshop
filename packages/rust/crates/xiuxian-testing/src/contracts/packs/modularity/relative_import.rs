//! Relative-import clarity heuristics for Rust modularity checks.

use proc_macro2::{LineColumn, Span};
use syn::spanned::Spanned;
use syn::{Item, ItemUse, UseTree};

/// Result of evaluating whether a file uses multi-hop relative imports.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum RelativeImportCheck {
    /// No clarity issue was detected.
    Clear,
    /// One top-level `use` item starts with repeated `super` hops.
    MultiHopRelativeImport(RelativeImport),
}

/// One detected multi-hop relative import.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RelativeImport {
    /// 1-based line number of the import.
    pub(crate) line_number: usize,
    /// Rendered source path that triggered the warning.
    pub(crate) rendered_path: String,
}

/// Check whether a file contains one `super::super` style import.
#[must_use]
pub(crate) fn check_relative_import_clarity(text: &str) -> RelativeImportCheck {
    let Ok(file) = syn::parse_file(text) else {
        return RelativeImportCheck::Clear;
    };

    file.items
        .into_iter()
        .find_map(|item| match item {
            Item::Use(item_use) => classify_item_use(&item_use),
            _ => None,
        })
        .map_or(
            RelativeImportCheck::Clear,
            RelativeImportCheck::MultiHopRelativeImport,
        )
}

fn classify_item_use(item_use: &ItemUse) -> Option<RelativeImport> {
    let repeated_super = use_tree_leading_super_hops(&item_use.tree);
    if repeated_super < 2 {
        return None;
    }

    Some(RelativeImport {
        line_number: span_start(item_use.span()).line,
        rendered_path: render_use_tree(&item_use.tree),
    })
}

fn use_tree_leading_super_hops(tree: &UseTree) -> usize {
    match tree {
        UseTree::Path(path) if path.ident == "super" => 1 + use_tree_leading_super_hops(&path.tree),
        UseTree::Group(group) => group
            .items
            .iter()
            .map(use_tree_leading_super_hops)
            .max()
            .unwrap_or(0),
        UseTree::Path(_) | UseTree::Name(_) | UseTree::Rename(_) | UseTree::Glob(_) => 0,
    }
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct SourceLocation {
    line: usize,
}
