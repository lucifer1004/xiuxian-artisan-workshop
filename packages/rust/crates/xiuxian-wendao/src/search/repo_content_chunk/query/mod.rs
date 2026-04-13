mod search;
#[cfg(test)]
#[path = "../../../../tests/unit/search/repo_content_chunk/query/mod.rs"]
mod tests;

pub(crate) use search::{
    RepoContentChunkCandidate, RepoContentChunkSearchError, RepoContentChunkSearchFilters,
    search_repo_content_chunks_with_filters,
};
#[cfg(test)]
pub(crate) use search::{
    build_repo_content_stage1_sql, candidate_path_key, compare_candidates, retained_window,
};
