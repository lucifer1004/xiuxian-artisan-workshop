//! High-level repository intelligence service orchestration.

mod analysis;
mod bootstrap;
mod cached;
mod helpers;
mod merge;
mod projection;
mod registry;
mod relation_dedupe;
mod search;
mod sync;

pub use analysis::{
    analyze_registered_repository, analyze_registered_repository_with_registry,
    analyze_repository_from_config, analyze_repository_from_config_with_registry,
};
pub use bootstrap::bootstrap_builtin_registry;
pub use cached::analyze_registered_repository_cached_with_registry;
pub use helpers::relation_kind_label;
pub use projection::*;
pub use registry::load_registered_repository;
pub use search::*;
pub use sync::*;

#[cfg(test)]
mod tests;
