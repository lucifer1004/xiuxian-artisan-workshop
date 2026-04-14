//! File-size and mixed-responsibility heuristics for Rust modularity checks.

use std::path::Path;

use syn::{Item, Visibility};

const MAX_EFFECTIVE_CODE_LINES: usize = 220;
const MAX_TOP_LEVEL_ITEMS: usize = 8;
const MIN_RESPONSIBILITY_GROUPS: usize = 3;
const MIN_PUBLIC_SURFACE_ITEMS: usize = 5;

/// Result of evaluating one Rust source file for file-bloat risk.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum FileBloatCheck {
    /// The file is within the bounded modularity thresholds.
    WithinBounds,
    /// The file should be skipped for this heuristic.
    Skipped,
    /// The file exceeds the bounded modularity thresholds.
    Bloated(FileBloatMetrics),
}

/// Metrics describing a bloated Rust source file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct FileBloatMetrics {
    /// Non-empty, non-comment, non-attribute lines.
    pub(crate) effective_code_lines: usize,
    /// Top-level non-import item count.
    pub(crate) top_level_items: usize,
    /// Distinct top-level responsibility groups detected.
    pub(crate) responsibility_groups: usize,
    /// Number of top-level items with visible public surface.
    pub(crate) public_surface_items: usize,
}

/// Check whether one Rust source file looks too large for one ownership seam.
#[must_use]
pub(crate) fn check_rust_file_bloat(path: &Path, text: &str) -> FileBloatCheck {
    let Some(file_name) = path.file_name().and_then(|name| name.to_str()) else {
        return FileBloatCheck::Skipped;
    };
    if matches!(file_name, "lib.rs" | "main.rs" | "mod.rs") {
        return FileBloatCheck::Skipped;
    }

    let Ok(file) = syn::parse_file(text) else {
        return FileBloatCheck::Skipped;
    };

    let mut groups = ResponsibilityGroups::default();
    let mut top_level_items = 0;
    let mut public_surface_items = 0;

    for item in &file.items {
        if matches!(item, Item::Use(_)) {
            continue;
        }

        top_level_items += 1;
        groups.observe(item);
        if item_has_visible_surface(item) {
            public_surface_items += 1;
        }
    }

    let effective_code_lines = count_effective_code_lines(text);
    let responsibility_groups = groups.count();

    if effective_code_lines < MAX_EFFECTIVE_CODE_LINES
        || top_level_items < MAX_TOP_LEVEL_ITEMS
        || (responsibility_groups < MIN_RESPONSIBILITY_GROUPS
            && public_surface_items < MIN_PUBLIC_SURFACE_ITEMS)
    {
        return FileBloatCheck::WithinBounds;
    }

    FileBloatCheck::Bloated(FileBloatMetrics {
        effective_code_lines,
        top_level_items,
        responsibility_groups,
        public_surface_items,
    })
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

fn item_has_visible_surface(item: &Item) -> bool {
    item_visibility(item).is_some_and(|vis| !matches!(vis, Visibility::Inherited))
}

fn item_visibility(item: &Item) -> Option<&Visibility> {
    match item {
        Item::Const(item) => Some(&item.vis),
        Item::Enum(item) => Some(&item.vis),
        Item::ExternCrate(item) => Some(&item.vis),
        Item::Fn(item) => Some(&item.vis),
        Item::Mod(item) => Some(&item.vis),
        Item::Static(item) => Some(&item.vis),
        Item::Struct(item) => Some(&item.vis),
        Item::Trait(item) => Some(&item.vis),
        Item::TraitAlias(item) => Some(&item.vis),
        Item::Type(item) => Some(&item.vis),
        Item::Union(item) => Some(&item.vis),
        Item::Use(item) => Some(&item.vis),
        _ => None,
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
struct ResponsibilityGroups(u8);

impl ResponsibilityGroups {
    const TYPES: u8 = 1 << 0;
    const FUNCTIONS: u8 = 1 << 1;
    const IMPLS: u8 = 1 << 2;
    const CONSTANTS: u8 = 1 << 3;
    const MODULES: u8 = 1 << 4;
    const MACROS_OR_FOREIGN: u8 = 1 << 5;

    fn observe(&mut self, item: &Item) {
        match item {
            Item::Struct(_)
            | Item::Enum(_)
            | Item::Trait(_)
            | Item::TraitAlias(_)
            | Item::Type(_)
            | Item::Union(_) => self.0 |= Self::TYPES,
            Item::Fn(_) => self.0 |= Self::FUNCTIONS,
            Item::Impl(_) => self.0 |= Self::IMPLS,
            Item::Const(_) | Item::Static(_) => self.0 |= Self::CONSTANTS,
            Item::Mod(_) => self.0 |= Self::MODULES,
            Item::ExternCrate(_) | Item::ForeignMod(_) | Item::Macro(_) | Item::Verbatim(_) => {
                self.0 |= Self::MACROS_OR_FOREIGN;
            }
            Item::Use(_) => {}
            _ => self.0 |= Self::MACROS_OR_FOREIGN,
        }
    }

    fn count(self) -> usize {
        self.0.count_ones() as usize
    }
}
