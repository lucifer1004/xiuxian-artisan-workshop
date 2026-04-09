mod hydrate;
mod results;
mod search;

#[cfg(test)]
#[path = "../../../../tests/unit/search/repo_entity/query/mod.rs"]
mod tests;

pub(crate) use results::{
    search_repo_entity_example_results, search_repo_entity_import_results,
    search_repo_entity_module_results, search_repo_entity_symbol_results,
};
pub(crate) use search::{RepoEntitySearchError, search_repo_entities};
