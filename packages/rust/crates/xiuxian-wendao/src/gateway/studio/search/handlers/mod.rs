//! Search backend integration for Studio API.

mod ast;
mod attachments;
mod autocomplete;
mod code_search;
mod definition;
mod index;
mod knowledge;
mod queries;
mod references;
mod symbols;

#[cfg(test)]
use self::queries::{
    AstSearchQuery, AttachmentSearchQuery, AutocompleteQuery, DefinitionResolveQuery,
    ReferenceSearchQuery, SearchQuery, SymbolSearchQuery,
};
pub use ast::search_ast;
pub use attachments::search_attachments;
pub use autocomplete::search_autocomplete;
pub use definition::search_definition;
pub use index::{build_ast_index, build_symbol_index};
pub use knowledge::{search_intent, search_knowledge};
pub use references::search_references;
pub use symbols::search_symbols;

#[cfg(test)]
use std::sync::Arc;

#[cfg(test)]
use axum::extract::{Query, State};

#[cfg(test)]
use crate::gateway::studio::repo_index::RepoIndexSnapshot;

#[cfg(test)]
use self::code_search::{
    CODE_CONTENT_EXCLUDE_GLOBS, is_supported_code_extension, parse_content_search_line,
    parse_repo_code_search_query, path_matches_language_filters, repo_navigation_target,
    truncate_content_search_snippet,
};

#[cfg(test)]
#[path = "../../../../../tests/unit/gateway/studio/search.rs"]
mod studio_search_tests;

#[cfg(test)]
mod tests;
