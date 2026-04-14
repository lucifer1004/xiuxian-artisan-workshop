//! Root-seam navigation hints for LLM-friendly Rust module layouts.

use std::collections::BTreeSet;
use std::path::Path;

use syn::{AttrStyle, File, Item, Visibility};

const MIN_CHILD_MODULES: usize = 3;

/// Result of evaluating whether a root module provides a first-hop hint.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum RootNavigationHintCheck {
    /// The file is not a root-seam candidate.
    NotApplicable,
    /// A root-level doc hint or visible re-export is present.
    HintPresent,
    /// The root seam declares several child modules but gives no first-hop hint.
    MissingHint(RootNavigationHintMetrics),
}

/// Metrics describing a root seam without a navigation hint.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RootNavigationHintMetrics {
    /// Number of declared child modules.
    pub(crate) child_modules: usize,
}

/// Check whether one root seam gives coding agents a first-hop hint.
#[must_use]
pub(crate) fn check_root_navigation_hint(path: &Path, text: &str) -> RootNavigationHintCheck {
    if !is_root_seam_candidate(path) {
        return RootNavigationHintCheck::NotApplicable;
    }

    let Ok(file) = syn::parse_file(text) else {
        return RootNavigationHintCheck::NotApplicable;
    };

    let child_modules = collect_child_modules(&file);
    if child_modules.len() < MIN_CHILD_MODULES {
        return RootNavigationHintCheck::HintPresent;
    }

    if has_root_doc_hint(&file) || has_visible_reexport(&file) {
        return RootNavigationHintCheck::HintPresent;
    }

    RootNavigationHintCheck::MissingHint(RootNavigationHintMetrics {
        child_modules: child_modules.len(),
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

fn has_root_doc_hint(file: &File) -> bool {
    file.attrs
        .iter()
        .any(|attr| matches!(attr.style, AttrStyle::Inner(_)) && attr.path().is_ident("doc"))
}

fn has_visible_reexport(file: &File) -> bool {
    file.items.iter().any(|item| match item {
        Item::Use(item_use) => !matches!(item_use.vis, Visibility::Inherited),
        _ => false,
    })
}
