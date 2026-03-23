use std::path::Path;

use tokio::runtime::Handle;
use xiuxian_vector::VectorStoreError;

use crate::gateway::studio::build_ast_index;
use crate::gateway::studio::types::AstSearchHit;
use crate::gateway::studio::types::UiProjectConfig;
use crate::search_plane::{
    BeginBuildDecision, SearchBuildLease, SearchCorpusKind, SearchPlaneService,
};

use super::schema::{local_symbol_batches, local_symbol_schema};

#[cfg(test)]
#[derive(Debug, thiserror::Error)]
pub(crate) enum LocalSymbolBuildError {
    #[error("local symbol build was not started for fingerprint `{0}`")]
    BuildRejected(String),
    #[error(transparent)]
    Storage(#[from] VectorStoreError),
}

pub(crate) fn ensure_local_symbol_index_started(
    service: &SearchPlaneService,
    project_root: &Path,
    config_root: &Path,
    projects: &[UiProjectConfig],
) {
    if projects.is_empty() {
        return;
    }

    let fingerprint = fingerprint_projects(project_root, config_root, projects);
    let decision = service.coordinator().begin_build(
        SearchCorpusKind::LocalSymbol,
        fingerprint,
        SearchCorpusKind::LocalSymbol.schema_version(),
    );
    let BeginBuildDecision::Started(lease) = decision else {
        return;
    };

    let build_projects = projects.to_vec();
    let build_project_root = project_root.to_path_buf();
    let build_config_root = config_root.to_path_buf();
    let service = service.clone();

    if let Ok(handle) = Handle::try_current() {
        handle.spawn(async move {
            let build: Result<Vec<AstSearchHit>, tokio::task::JoinError> =
                tokio::task::spawn_blocking(move || {
                    build_ast_index(
                        build_project_root.as_path(),
                        build_config_root.as_path(),
                        &build_projects,
                    )
                })
                .await;

            match build {
                Ok(hits) => {
                    service.coordinator().update_progress(&lease, 0.45);
                    if let Err(error) = write_local_symbol_epoch(&service, &lease, &hits).await {
                        service.coordinator().fail_build(
                            &lease,
                            format!("local symbol epoch write failed: {error}"),
                        );
                        return;
                    }
                    service.coordinator().update_progress(&lease, 1.0);
                    service.publish_ready_and_maintain(
                        &lease,
                        hits.len() as u64,
                        fragment_count_from_len(hits.len()),
                    );
                }
                Err(error) => {
                    service.coordinator().fail_build(
                        &lease,
                        format!("local symbol background build panicked: {error}"),
                    );
                }
            }
        });
    } else {
        service.coordinator().fail_build(
            &lease,
            "Tokio runtime unavailable for local symbol index build",
        );
    }
}

async fn write_local_symbol_epoch(
    service: &SearchPlaneService,
    lease: &SearchBuildLease,
    hits: &[crate::gateway::studio::types::AstSearchHit],
) -> Result<(), VectorStoreError> {
    let store = service.open_store(SearchCorpusKind::LocalSymbol).await?;
    let table_name = service.table_name(SearchCorpusKind::LocalSymbol, lease.epoch);
    let schema = local_symbol_schema();
    let batches = local_symbol_batches(hits)?;
    store
        .replace_record_batches(table_name.as_str(), schema, batches)
        .await?;
    store
        .create_inverted_index(table_name.as_str(), "search_text", None)
        .await?;
    Ok(())
}

#[cfg(test)]
pub(crate) async fn publish_local_symbol_hits(
    service: &SearchPlaneService,
    fingerprint: &str,
    hits: &[AstSearchHit],
) -> Result<(), LocalSymbolBuildError> {
    let lease = match service.coordinator().begin_build(
        SearchCorpusKind::LocalSymbol,
        fingerprint,
        SearchCorpusKind::LocalSymbol.schema_version(),
    ) {
        BeginBuildDecision::Started(lease) => lease,
        BeginBuildDecision::AlreadyReady(_) | BeginBuildDecision::AlreadyIndexing(_) => {
            return Err(LocalSymbolBuildError::BuildRejected(
                fingerprint.to_string(),
            ));
        }
    };

    match write_local_symbol_epoch(service, &lease, hits).await {
        Ok(()) => {
            service.publish_ready_and_maintain(
                &lease,
                hits.len() as u64,
                fragment_count_from_len(hits.len()),
            );
            Ok(())
        }
        Err(error) => {
            service
                .coordinator()
                .fail_build(&lease, format!("local symbol epoch write failed: {error}"));
            Err(LocalSymbolBuildError::Storage(error))
        }
    }
}

fn fragment_count_from_len(hit_count: usize) -> u64 {
    if hit_count == 0 {
        1
    } else {
        hit_count.div_ceil(1_000) as u64
    }
}

fn fingerprint_projects(
    project_root: &Path,
    config_root: &Path,
    projects: &[UiProjectConfig],
) -> String {
    let mut hasher = blake3::Hasher::new();
    hasher.update(project_root.to_string_lossy().as_bytes());
    hasher.update(config_root.to_string_lossy().as_bytes());
    for project in projects {
        hasher.update(project.name.as_bytes());
        hasher.update(project.root.as_bytes());
        for dir in &project.dirs {
            hasher.update(dir.as_bytes());
        }
    }
    hasher.finalize().to_hex().to_string()
}
