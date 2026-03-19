mod handlers;
mod project_scope;
mod source_index;

pub(crate) use handlers::{build_ast_index, build_symbol_index};
pub(super) use handlers::{
    search_ast, search_attachments, search_autocomplete, search_definition, search_knowledge,
    search_references, search_symbols,
};
