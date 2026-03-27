//! Search backend integration for Studio API.

pub mod definition;
pub mod handlers;
pub mod observation_hints;
pub mod project_scope;
pub mod source_index;
pub mod support;

pub use handlers::{
    build_symbol_index, search_ast, search_ast_hits_arrow, search_attachments,
    search_attachments_hits_arrow, search_autocomplete, search_definition, search_index_status,
    search_intent, search_intent_hits_arrow, search_knowledge, search_references,
    search_references_hits_arrow, search_symbols, search_symbols_hits_arrow,
};

#[cfg(test)]
pub use handlers::build_ast_index;
