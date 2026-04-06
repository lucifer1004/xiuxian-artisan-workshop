mod batch;
mod buffered;
mod dispatch;
mod entity;
mod orchestration;
mod search;

#[cfg(test)]
pub(crate) use self::buffered::RepoSearchResultLimits;
#[cfg(test)]
pub(crate) use self::dispatch::{collect_repo_search_targets, repo_search_parallelism};
#[cfg(test)]
pub(crate) use self::entity::search_repo_entity_hits_for_query;
#[cfg(test)]
pub(crate) use self::orchestration::search_repo_code_outcome;
pub(crate) use self::orchestration::search_repo_intent_outcome;
pub(crate) use self::search::search_repo_content_batch;
#[cfg(test)]
pub(crate) use self::search::search_repo_content_hits_for_query;
