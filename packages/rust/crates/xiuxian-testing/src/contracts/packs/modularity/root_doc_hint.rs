//! Root-doc specificity checks for LLM-friendly Rust module layouts.

use std::collections::BTreeSet;
use std::path::Path;

use proc_macro2::{LineColumn, Span};
use syn::spanned::Spanned;
use syn::{AttrStyle, Expr, ExprLit, File, Item, Lit, Meta, Visibility};

const MIN_CHILD_MODULES: usize = 3;
const DOC_PREVIEW_LIMIT: usize = 80;

/// Result of evaluating whether a doc-only root seam names a child module.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum RootDocHintCheck {
    /// The file is not a root-seam candidate for this rule.
    NotApplicable,
    /// The root seam already exposes a specific enough first-hop hint.
    SpecificHint,
    /// The root seam relies on docs but never names any child module.
    DocWithoutChildModuleName(RootDocHintMetrics),
}

/// Metrics for a root doc hint that stays too generic.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RootDocHintMetrics {
    /// Number of declared child modules.
    pub(crate) child_modules: usize,
    /// 1-based line number of the first root doc attribute.
    pub(crate) line_number: usize,
    /// Short preview of the root doc text.
    pub(crate) doc_preview: String,
}

/// Check whether a doc-only root seam names at least one child module.
#[must_use]
pub(crate) fn check_root_doc_hint(path: &Path, text: &str) -> RootDocHintCheck {
    if !is_root_seam_candidate(path) {
        return RootDocHintCheck::NotApplicable;
    }

    let Ok(file) = syn::parse_file(text) else {
        return RootDocHintCheck::NotApplicable;
    };

    let child_modules = collect_child_modules(&file);
    if child_modules.len() < MIN_CHILD_MODULES {
        return RootDocHintCheck::SpecificHint;
    }
    if has_visible_reexport(&file) {
        return RootDocHintCheck::SpecificHint;
    }

    let Some(root_doc) = collect_root_doc(&file) else {
        return RootDocHintCheck::NotApplicable;
    };

    let doc_text = root_doc.text.to_lowercase();
    if child_modules
        .iter()
        .any(|module_name| doc_text.contains(&module_name.to_lowercase()))
    {
        return RootDocHintCheck::SpecificHint;
    }

    RootDocHintCheck::DocWithoutChildModuleName(RootDocHintMetrics {
        child_modules: child_modules.len(),
        line_number: root_doc.line_number,
        doc_preview: root_doc.preview,
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

fn has_visible_reexport(file: &File) -> bool {
    file.items.iter().any(|item| match item {
        Item::Use(item_use) => !matches!(item_use.vis, Visibility::Inherited),
        _ => false,
    })
}

fn collect_root_doc(file: &File) -> Option<RootDoc> {
    let doc_attrs = file
        .attrs
        .iter()
        .filter(|attr| matches!(attr.style, AttrStyle::Inner(_)) && attr.path().is_ident("doc"))
        .collect::<Vec<_>>();
    let first_attr = doc_attrs.first()?;

    let text = doc_attrs
        .iter()
        .filter_map(|attr| extract_doc_text(attr))
        .collect::<Vec<_>>()
        .join(" ");
    if text.trim().is_empty() {
        return None;
    }

    Some(RootDoc {
        line_number: span_start(first_attr.span()).line,
        preview: doc_preview(&text),
        text,
    })
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

fn doc_preview(text: &str) -> String {
    let trimmed = text.trim();
    let mut preview = trimmed.chars().take(DOC_PREVIEW_LIMIT).collect::<String>();
    if trimmed.chars().count() > DOC_PREVIEW_LIMIT {
        preview.push_str("...");
    }
    preview
}

fn span_start(span: Span) -> SourceLocation {
    let LineColumn { line, .. } = span.start();
    SourceLocation { line: line.max(1) }
}

#[derive(Debug, Clone)]
struct RootDoc {
    line_number: usize,
    preview: String,
    text: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct SourceLocation {
    line: usize,
}
