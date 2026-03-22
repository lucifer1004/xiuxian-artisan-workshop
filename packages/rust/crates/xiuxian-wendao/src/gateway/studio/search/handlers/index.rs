use std::path::Path;

use crate::gateway::studio::types::{AstSearchHit, UiProjectConfig};
use crate::unified_symbol::UnifiedSymbolIndex;

use super::super::source_index;

pub fn build_ast_index(
    project_root: &Path,
    config_root: &Path,
    projects: &[UiProjectConfig],
) -> Vec<AstSearchHit> {
    source_index::build_ast_index(project_root, config_root, projects)
}

pub fn build_symbol_index(
    project_root: &Path,
    config_root: &Path,
    projects: &[UiProjectConfig],
) -> UnifiedSymbolIndex {
    source_index::build_symbol_index(project_root, config_root, projects)
}
