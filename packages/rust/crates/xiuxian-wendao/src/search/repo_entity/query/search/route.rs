use std::collections::HashSet;

use crate::gateway::studio::types::SearchHit;
use crate::search::SearchPlaneService;
use crate::search::repo_entity::query::hydrate::hydrate_repo_entity_hits;
use crate::search::repo_entity::query::search::{
    RepoEntitySearchError, prepare_repo_entity_search,
};

pub(crate) async fn search_repo_entities(
    service: &SearchPlaneService,
    repo_id: &str,
    query: &str,
    language_filters: &HashSet<String>,
    kind_filters: &HashSet<String>,
    limit: usize,
) -> Result<Vec<SearchHit>, RepoEntitySearchError> {
    let Some(prepared) = prepare_repo_entity_search(
        service,
        repo_id,
        query,
        language_filters,
        kind_filters,
        limit,
    )
    .await?
    else {
        return Ok(Vec::new());
    };
    let hits: Vec<SearchHit> = hydrate_repo_entity_hits(
        service.search_engine(),
        prepared.engine_table_name.as_str(),
        prepared.candidates,
    )
    .await?;
    service.record_query_telemetry(
        crate::search::SearchCorpusKind::RepoEntity,
        prepared
            .telemetry
            .finish(prepared.source, Some(repo_id.to_string()), hits.len()),
    );
    Ok(hits)
}
