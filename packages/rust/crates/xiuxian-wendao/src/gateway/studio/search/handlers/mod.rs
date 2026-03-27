//! Search backend integration for Studio API.

mod arrow_transport;
mod ast;
mod attachments;
mod autocomplete;
#[path = "code_search/mod.rs"]
mod code_search;
mod definition;
mod index;
mod knowledge;
mod queries;
mod references;
mod status;
mod symbols;
#[cfg(test)]
mod test_prelude;

pub use ast::{search_ast, search_ast_hits_arrow};
pub use attachments::{search_attachments, search_attachments_hits_arrow};
pub use autocomplete::search_autocomplete;
pub use definition::search_definition;
#[cfg(test)]
pub use index::build_ast_index;
pub use index::build_symbol_index;
pub use knowledge::{search_intent, search_intent_hits_arrow, search_knowledge};
pub use references::{search_references, search_references_hits_arrow};
pub use status::search_index_status;
pub use symbols::{search_symbols, search_symbols_hits_arrow};

#[cfg(test)]
#[path = "../../../../../tests/unit/gateway/studio/search.rs"]
mod studio_search_tests;

#[cfg(test)]
mod tests;
