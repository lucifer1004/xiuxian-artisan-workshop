use super::super::code_search::{build_repo_content_search_hits, build_repo_entity_search_hits};
use crate::gateway::studio::router::{
    StudioApiError, StudioState, configured_repositories, configured_repository,
    map_repo_intelligence_error,
};
use crate::gateway::studio::types::SearchHit;
use crate::search_plane::RepoSearchAvailability;

#[derive(Debug, Default)]
pub(super) struct RepoIntentMerge {
    pub(super) hits: Vec<SearchHit>,
    pub(super) pending_repos: Vec<String>,
    pub(super) skipped_repos: Vec<String>,
}

pub(super) async fn build_repo_intent_merge(
    studio: &StudioState,
    raw_query: &str,
    repo_hint: Option<&str>,
    limit: usize,
) -> Result<RepoIntentMerge, StudioApiError> {
    let repositories = if let Some(repo_id) = repo_hint {
        vec![configured_repository(studio, repo_id).map_err(map_repo_intelligence_error)?]
    } else {
        configured_repositories(studio)
    };

    let mut merge = RepoIntentMerge::default();
    for repository in repositories {
        let publication_state = studio
            .search_plane
            .repo_search_publication_state(repository.id.as_str())
            .await;
        if !publication_state.is_searchable() {
            match publication_state.availability {
                RepoSearchAvailability::Skipped => {
                    merge.skipped_repos.push(repository.id.clone());
                }
                RepoSearchAvailability::Pending => {
                    merge.pending_repos.push(repository.id.clone());
                }
                RepoSearchAvailability::Searchable => {}
            }
            continue;
        }
        if publication_state.entity_published {
            merge.hits.extend(
                build_repo_entity_search_hits(studio, repository.id.as_str(), raw_query, limit)
                    .await?,
            );
        }
        if publication_state.content_published {
            merge.hits.extend(
                build_repo_content_search_hits(studio, repository.id.as_str(), raw_query, limit)
                    .await?,
            );
        }
    }

    Ok(merge)
}
