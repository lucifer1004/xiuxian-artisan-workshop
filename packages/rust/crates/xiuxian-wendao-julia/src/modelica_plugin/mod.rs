mod analysis;
mod discovery;
mod entry;
mod parser_summary;
mod parsing;
mod relations;
mod types;

pub use entry::{ModelicaRepoIntelligencePlugin, register_modelica_into};
pub use parser_summary::set_linked_modelica_parser_summary_base_url_for_tests;
