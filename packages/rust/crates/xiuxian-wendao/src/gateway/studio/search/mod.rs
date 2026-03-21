//! Search backend integration for Studio API.

pub mod definition;
pub mod handlers;
pub mod observation_hints;
pub mod project_scope;
pub mod source_index;
pub mod support;

pub use handlers::{
    build_ast_index, build_symbol_index, search_ast, search_attachments, search_autocomplete,
    search_definition, search_intent, search_knowledge, search_references, search_symbols,
};
