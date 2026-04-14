mod analysis;
mod discovery;
mod entry;
mod incremental;
mod parser_summary;
mod parsing;
mod relations;
mod types;

pub use entry::{ModelicaRepoIntelligencePlugin, register_modelica_into};
pub use incremental::{
    modelica_package_incremental_semantic_fingerprint_for_repository,
    modelica_parser_summary_allows_safe_incremental_file_for_repository,
    modelica_parser_summary_allows_safe_package_incremental_file_for_repository,
    modelica_parser_summary_allows_safe_root_package_incremental_file_for_repository,
    modelica_parser_summary_root_package_name_matches_repository_context,
    modelica_root_package_incremental_semantic_fingerprint_for_repository,
};
pub use parser_summary::{
    modelica_parser_summary_file_semantic_fingerprint_for_repository,
    set_linked_modelica_parser_summary_base_url_for_tests,
};
