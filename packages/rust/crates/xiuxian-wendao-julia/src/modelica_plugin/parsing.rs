//! Modelica parsing bridged through the native `WendaoCodeParser.jl` summary
//! route.

use xiuxian_wendao_core::repo_intelligence::RegisteredRepository;
use xiuxian_wendao_core::repo_intelligence::RepoIntelligenceError;

use super::parser_summary::fetch_modelica_parser_file_summary_blocking_for_repository;
use super::types::{ParsedDeclaration, ParsedImport};

/// Parse the package or class name from Modelica source through the native
/// parser-summary contract.
pub(crate) fn parse_package_name_for_repository(
    repository: &RegisteredRepository,
    source_id: &str,
    contents: &str,
) -> Result<Option<String>, RepoIntelligenceError> {
    Ok(
        fetch_modelica_parser_file_summary_blocking_for_repository(
            repository, source_id, contents,
        )?
        .class_name,
    )
}

/// Check if the source contains a Documentation annotation.
pub(crate) fn contains_documentation_annotation(contents: &str) -> bool {
    contents.contains("Documentation(")
}

/// Parse import statements from Modelica source through the native
/// parser-summary contract.
pub(crate) fn parse_imports_for_repository(
    repository: &RegisteredRepository,
    source_id: &str,
    contents: &str,
) -> Result<Vec<ParsedImport>, RepoIntelligenceError> {
    Ok(
        fetch_modelica_parser_file_summary_blocking_for_repository(
            repository, source_id, contents,
        )?
        .imports,
    )
}

/// Parse symbol declarations from Modelica source through the native
/// parser-summary contract.
pub(crate) fn parse_symbol_declarations_for_repository(
    repository: &RegisteredRepository,
    source_id: &str,
    contents: &str,
) -> Result<Vec<ParsedDeclaration>, RepoIntelligenceError> {
    Ok(
        fetch_modelica_parser_file_summary_blocking_for_repository(
            repository, source_id, contents,
        )?
        .declarations,
    )
}

#[cfg(test)]
#[path = "../../tests/unit/plugin/parsing.rs"]
mod tests;
