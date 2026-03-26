use tokio::task::JoinSet;

use crate::gateway::studio::router::StudioApiError;
use crate::gateway::studio::types::SearchHit;
use crate::search_plane::SearchPlaneService;

use crate::gateway::studio::search::handlers::code_search::query::RepoSearchResultLimits;
use crate::gateway::studio::search::handlers::code_search::search::repo_search::{
    search_repo_content_hits, search_repo_entity_hits,
};
use crate::gateway::studio::search::handlers::code_search::types::RepoSearchTarget;

pub(super) fn spawn_repo_code_search_task(
    join_set: &mut JoinSet<Result<Vec<SearchHit>, StudioApiError>>,
    search_plane: SearchPlaneService,
    target: RepoSearchTarget,
    raw_query: String,
    per_repo_limits: RepoSearchResultLimits,
) {
    join_set.spawn(async move {
        let mut repository_hits = if target.publication_state.entity_published {
            search_repo_entity_hits(
                &search_plane,
                target.repo_id.as_str(),
                raw_query.as_str(),
                per_repo_limits.entity_limit,
            )
            .await?
        } else {
            Vec::new()
        };

        if repository_hits.is_empty() && target.publication_state.content_published {
            repository_hits.extend(
                search_repo_content_hits(
                    &search_plane,
                    target.repo_id.as_str(),
                    raw_query.as_str(),
                    per_repo_limits.content_limit,
                )
                .await?,
            );
        }

        Ok(repository_hits)
    });
}
