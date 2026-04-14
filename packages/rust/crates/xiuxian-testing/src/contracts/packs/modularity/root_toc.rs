//! Root-module table-of-contents heuristics for folder-first Rust layouts.

use std::path::Path;

use proc_macro2::{LineColumn, Span};
use syn::Item;
use syn::spanned::Spanned;

const MIN_CHILD_MODULES: usize = 3;
const MIN_EFFECTIVE_CODE_LINES: usize = 50;

/// Result of evaluating whether a root module remains a clear table of contents.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum RootModuleTocCheck {
    /// The file is not a folder-root module candidate.
    NotApplicable,
    /// The file is a clear root module or facade.
    ClearToc,
    /// The file declares several child modules but still behaves like a sink.
    MixedRoot(RootModuleTocMetrics),
}

/// Metrics describing one root module that lost its TOC role.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RootModuleTocMetrics {
    /// Number of child module declarations.
    pub(crate) child_modules: usize,
    /// Non-empty, non-comment, non-attribute code lines.
    pub(crate) effective_code_lines: usize,
    /// 1-based line number of the first implementation item.
    pub(crate) first_implementation_line: usize,
    /// Human-readable description of the first implementation item.
    pub(crate) first_implementation_item: String,
}

/// Check whether one folder-root Rust module still reads like a TOC.
#[must_use]
pub(crate) fn check_root_module_toc(path: &Path, text: &str) -> RootModuleTocCheck {
    if !is_folder_root_candidate(path) {
        return RootModuleTocCheck::NotApplicable;
    }

    let Ok(file) = syn::parse_file(text) else {
        return RootModuleTocCheck::NotApplicable;
    };

    let mut child_modules = 0;
    let mut first_implementation = None;

    for item in file.items {
        match item {
            Item::Mod(item_mod) if item_mod.content.is_none() => {
                child_modules += 1;
            }
            Item::Use(_) => {}
            other => {
                first_implementation.get_or_insert_with(|| RootImplementationItem {
                    line_number: span_start(other.span()).line,
                    description: describe_item(&other),
                });
            }
        }
    }

    let effective_code_lines = count_effective_code_lines(text);
    let Some(first_implementation) = first_implementation else {
        return RootModuleTocCheck::ClearToc;
    };

    if child_modules < MIN_CHILD_MODULES || effective_code_lines < MIN_EFFECTIVE_CODE_LINES {
        return RootModuleTocCheck::ClearToc;
    }

    RootModuleTocCheck::MixedRoot(RootModuleTocMetrics {
        child_modules,
        effective_code_lines,
        first_implementation_line: first_implementation.line_number,
        first_implementation_item: first_implementation.description,
    })
}

fn is_folder_root_candidate(path: &Path) -> bool {
    let Some(file_name) = path.file_name().and_then(|name| name.to_str()) else {
        return false;
    };
    if matches!(file_name, "lib.rs" | "main.rs" | "mod.rs") {
        return false;
    }
    if path.extension().and_then(|ext| ext.to_str()) != Some("rs") {
        return false;
    }
    path.with_extension("").is_dir()
}

fn count_effective_code_lines(text: &str) -> usize {
    text.lines()
        .map(str::trim)
        .filter(|line| {
            !line.is_empty()
                && !line.starts_with("//")
                && !line.starts_with("/*")
                && !line.starts_with('*')
                && !line.starts_with("*/")
                && !line.starts_with("#[")
                && !line.starts_with("#![")
        })
        .count()
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
    let LineColumn { line, .. } = span.start();
    SourceLocation { line: line.max(1) }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct RootImplementationItem {
    line_number: usize,
    description: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct SourceLocation {
    line: usize,
}
