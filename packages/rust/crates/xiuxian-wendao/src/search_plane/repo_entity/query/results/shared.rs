use std::collections::BTreeMap;

use crate::search_plane::SearchPlaneService;
use crate::search_plane::repo_entity::query::hydrate::{
    load_hydrated_rows_by_id, typed_repo_entity_columns,
};
use crate::search_plane::repo_entity::query::search::{
    HydratedRepoEntityRow, RepoEntityCandidate, RepoEntitySearchError,
};

pub(super) async fn load_hydrated_rows(
    service: &SearchPlaneService,
    engine_table_name: &str,
    candidates: &[RepoEntityCandidate],
) -> Result<BTreeMap<String, HydratedRepoEntityRow>, RepoEntitySearchError> {
    let ids = candidates
        .iter()
        .map(|candidate| candidate.id.clone())
        .collect::<Vec<_>>();
    load_hydrated_rows_by_id(
        service.search_engine(),
        engine_table_name,
        ids.as_slice(),
        typed_repo_entity_columns().as_slice(),
    )
    .await
}
