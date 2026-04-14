//! Root-facade child-module visibility checks for LLM-friendly Rust layouts.

use std::path::Path;

use proc_macro2::{LineColumn, Span};
use syn::spanned::Spanned;
use syn::{Item, Visibility};

/// Result of evaluating whether a folder-root seam keeps child modules private.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum RootChildVisibilityCheck {
    /// The file is not a folder-root seam candidate.
    NotApplicable,
    /// The folder-root seam keeps child modules private.
    PrivateChildrenOnly,
    /// One child module declaration is visible from the root seam.
    VisibleChildModule(VisibleChildModule),
}

/// One visible child module declaration found at the root seam.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct VisibleChildModule {
    /// 1-based line number of the visible child module declaration.
    pub(crate) line_number: usize,
    /// Child module name.
    pub(crate) module_name: String,
    /// Rendered visibility token.
    pub(crate) visibility: String,
}

/// Check whether a folder-root seam exposes one child module declaration.
#[must_use]
pub(crate) fn check_root_child_visibility(path: &Path, text: &str) -> RootChildVisibilityCheck {
    if !is_folder_root_candidate(path) {
        return RootChildVisibilityCheck::NotApplicable;
    }

    let Ok(file) = syn::parse_file(text) else {
        return RootChildVisibilityCheck::NotApplicable;
    };

    for item in file.items {
        let Item::Mod(item_mod) = item else {
            continue;
        };
        if matches!(item_mod.vis, Visibility::Inherited) {
            continue;
        }

        return RootChildVisibilityCheck::VisibleChildModule(VisibleChildModule {
            line_number: span_start(item_mod.span()).line,
            module_name: item_mod.ident.to_string(),
            visibility: render_visibility(&item_mod.vis),
        });
    }

    RootChildVisibilityCheck::PrivateChildrenOnly
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

fn render_visibility(visibility: &Visibility) -> String {
    match visibility {
        Visibility::Inherited => "private".to_string(),
        Visibility::Public(_) => "pub".to_string(),
        Visibility::Restricted(restricted) => {
            let path = path_to_string(&restricted.path);
            if restricted.in_token.is_some() {
                format!("pub(in {path})")
            } else {
                format!("pub({path})")
            }
        }
    }
}

fn path_to_string(path: &syn::Path) -> String {
    path.segments
        .iter()
        .map(|segment| segment.ident.to_string())
        .collect::<Vec<_>>()
        .join("::")
}

fn span_start(span: Span) -> SourceLocation {
    let LineColumn { line, .. } = span.start();
    SourceLocation { line: line.max(1) }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct SourceLocation {
    line: usize,
}
