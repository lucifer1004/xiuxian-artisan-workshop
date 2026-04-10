use xiuxian_wendao_core::repo_intelligence::{RegisteredRepository, RepoIntelligenceError};

use super::fetch::fetch_julia_parser_file_summary_blocking_for_repository;

/// Return whether one Julia source file is safe for leaf-only incremental
/// analysis under the native parser-summary contract.
///
/// Safe incremental files must not declare a module, add imports, or add
/// includes because those changes widen repository structure instead of staying
/// leaf-local.
///
/// # Errors
///
/// Returns [`RepoIntelligenceError`] when the repository does not declare a
/// usable parser-summary client or when the remote parser-summary request
/// fails.
pub fn julia_parser_summary_allows_safe_incremental_file_for_repository(
    repository: &RegisteredRepository,
    source_id: &str,
    source_text: &str,
) -> Result<bool, RepoIntelligenceError> {
    let summary = fetch_julia_parser_file_summary_blocking_for_repository(
        repository,
        source_id,
        source_text,
    )?;
    Ok(summary.module_name.is_none() && summary.imports.is_empty() && summary.includes.is_empty())
}

#[cfg(test)]
#[path = "../../../tests/unit/plugin/parser_summary/incremental.rs"]
mod tests;
