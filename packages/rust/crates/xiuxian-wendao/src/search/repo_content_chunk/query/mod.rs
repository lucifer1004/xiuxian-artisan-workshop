mod search;
#[cfg(test)]
#[path = "../../../../tests/unit/search/repo_content_chunk/query/mod.rs"]
mod tests;

#[cfg(test)]
pub(crate) use search::{
    RepoContentChunkCandidate, build_repo_content_stage1_sql, candidate_path_key,
    compare_candidates, retained_window,
};
pub(crate) use search::{
    RepoContentChunkSearchError, RepoContentChunkSearchFilters,
    search_repo_content_chunks_with_filters,
};
