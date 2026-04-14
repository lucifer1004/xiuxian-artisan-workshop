//! Native Rust syntax helpers for the modularity rule pack.

use proc_macro2::{LineColumn, Span};
use syn::spanned::Spanned;
use syn::{Item, ItemUse, UseTree, Visibility};

/// Result of checking whether one `mod.rs` file is interface-only.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ModInterfaceCheck {
    /// The file contains only allowed interface items.
    InterfaceOnly,
    /// The file contains one top-level implementation item.
    NonInterfaceItem(NonInterfaceItem),
    /// The file could not be parsed as Rust syntax.
    ParseFailure(ParseFailure),
}

/// One top-level implementation item detected in `mod.rs`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct NonInterfaceItem {
    /// 1-based line number of the first offending item.
    pub(crate) line_number: usize,
    /// Human-readable description of the item.
    pub(crate) description: String,
}

/// One native Rust syntax parse failure.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ParseFailure {
    /// 1-based line number where parsing failed.
    pub(crate) line_number: usize,
    /// 1-based column number where parsing failed.
    pub(crate) column_number: usize,
    /// Parser error message.
    pub(crate) message: String,
}

/// Check whether the file contents satisfy the interface-only `mod.rs` rule.
#[must_use]
pub(crate) fn check_mod_rs_interface(text: &str) -> ModInterfaceCheck {
    match syn::parse_file(text) {
        Ok(file) => file
            .items
            .into_iter()
            .find_map(classify_non_interface_item)
            .map_or(
                ModInterfaceCheck::InterfaceOnly,
                ModInterfaceCheck::NonInterfaceItem,
            ),
        Err(error) => {
            let location = span_start(error.span());
            ModInterfaceCheck::ParseFailure(ParseFailure {
                line_number: location.line,
                column_number: location.column,
                message: error.to_string(),
            })
        }
    }
}

fn classify_non_interface_item(item: Item) -> Option<NonInterfaceItem> {
    match item {
        Item::Use(item_use) => classify_use_item(&item_use),
        Item::Mod(item_mod) if item_mod.content.is_none() => classify_mod_declaration(&item_mod),
        other => Some(NonInterfaceItem {
            line_number: span_start(other.span()).line,
            description: describe_item(&other),
        }),
    }
}

fn classify_mod_declaration(item_mod: &syn::ItemMod) -> Option<NonInterfaceItem> {
    if matches!(item_mod.vis, Visibility::Inherited) {
        return None;
    }

    Some(NonInterfaceItem {
        line_number: span_start(item_mod.span()).line,
        description: format!("visible module declaration `{}`", item_mod.ident),
    })
}

fn classify_use_item(item_use: &ItemUse) -> Option<NonInterfaceItem> {
    if matches!(item_use.vis, Visibility::Inherited) {
        return Some(NonInterfaceItem {
            line_number: span_start(item_use.span()).line,
            description: "private `use` import".to_string(),
        });
    }

    if contains_glob_use(&item_use.tree) {
        return Some(NonInterfaceItem {
            line_number: span_start(item_use.span()).line,
            description: "glob re-export".to_string(),
        });
    }

    None
}

fn contains_glob_use(tree: &UseTree) -> bool {
    match tree {
        UseTree::Path(path) => contains_glob_use(&path.tree),
        UseTree::Group(group) => group.items.iter().any(contains_glob_use),
        UseTree::Glob(_) => true,
        UseTree::Name(_) | UseTree::Rename(_) => false,
    }
}

fn describe_item(item: &Item) -> String {
    match item {
        Item::Const(item) => format!("const `{}`", item.ident),
        Item::Enum(item) => format!("enum `{}`", item.ident),
        Item::ExternCrate(item) => format!("extern crate `{}`", item.ident),
        Item::Fn(item) => format!("function `{}`", item.sig.ident),
        Item::ForeignMod(_) => "foreign module block".to_string(),
        Item::Impl(_) => "impl block".to_string(),
        Item::Macro(item) => format!("macro invocation `{}`", path_to_string(&item.mac.path)),
        Item::Mod(item) => format!("inline module `{}`", item.ident),
        Item::Static(item) => format!("static `{}`", item.ident),
        Item::Struct(item) => format!("struct `{}`", item.ident),
        Item::Trait(item) => format!("trait `{}`", item.ident),
        Item::TraitAlias(item) => format!("trait alias `{}`", item.ident),
        Item::Type(item) => format!("type alias `{}`", item.ident),
        Item::Union(item) => format!("union `{}`", item.ident),
        Item::Use(_) => "use statement".to_string(),
        Item::Verbatim(_) | _ => "unclassified Rust item".to_string(),
    }
}

fn path_to_string(path: &syn::Path) -> String {
    let rendered = path
        .segments
        .iter()
        .map(|segment| segment.ident.to_string())
        .collect::<Vec<_>>()
        .join("::");
    if rendered.is_empty() {
        "<macro>".to_string()
    } else {
        rendered
    }
}

fn span_start(span: Span) -> SourceLocation {
    let LineColumn { line, column } = span.start();
    SourceLocation {
        line: line.max(1),
        column: column + 1,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct SourceLocation {
    line: usize,
    column: usize,
}
