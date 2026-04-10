mod contract;
mod fetch;
mod transport;
mod types;

pub(crate) use fetch::{
    fetch_modelica_parser_file_summary_blocking_for_repository,
    validate_modelica_parser_summary_preflight_for_repository,
};
pub use transport::set_linked_modelica_parser_summary_base_url_for_tests;
