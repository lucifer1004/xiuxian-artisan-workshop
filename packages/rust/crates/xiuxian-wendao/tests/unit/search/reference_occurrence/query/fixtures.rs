use std::path::PathBuf;

#[cfg(feature = "duckdb")]
use std::fs;

use crate::gateway::studio::types::{ReferenceSearchHit, StudioNavigationTarget};
use crate::search::reference_occurrence::schema::{
    reference_occurrence_batches, reference_occurrence_schema,
};
use crate::search::{
    BeginBuildDecision, SearchCorpusKind, SearchMaintenancePolicy, SearchManifestKeyspace,
    SearchPlaneService,
};
#[cfg(feature = "duckdb")]
use crate::set_link_graph_wendao_config_override;
use xiuxian_vector::ColumnarScanOptions;

pub(super) fn fixture_service(temp_dir: &tempfile::TempDir) -> SearchPlaneService {
    SearchPlaneService::with_paths(
        PathBuf::from("/tmp/project"),
        temp_dir.path().join("search_plane"),
        SearchManifestKeyspace::new("xiuxian:test:reference_occurrence"),
        SearchMaintenancePolicy::default(),
    )
}

#[cfg(feature = "duckdb")]
pub(super) fn write_search_duckdb_runtime_override(
    body: &str,
) -> Result<tempfile::TempDir, Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let config_path = temp.path().join("wendao.toml");
    fs::write(&config_path, body)?;
    set_link_graph_wendao_config_override(&config_path.to_string_lossy());
    Ok(temp)
}

pub(super) fn sample_hit(name: &str, path: &str, line: usize) -> ReferenceSearchHit {
    ReferenceSearchHit {
        name: name.to_string(),
        path: path.to_string(),
        language: "rust".to_string(),
        crate_name: "kernel".to_string(),
        project_name: None,
        root_label: None,
        line,
        column: 5,
        line_text: format!("let _value = {name};"),
        navigation_target: StudioNavigationTarget {
            path: path.to_string(),
            category: "doc".to_string(),
            project_name: None,
            root_label: None,
            line: Some(line),
            line_end: Some(line),
            column: Some(5),
        },
        score: 0.0,
    }
}

pub(super) async fn publish_reference_hits(
    service: &SearchPlaneService,
    build_id: &str,
    hits: &[ReferenceSearchHit],
) {
    let lease = match service.coordinator().begin_build(
        SearchCorpusKind::ReferenceOccurrence,
        build_id,
        SearchCorpusKind::ReferenceOccurrence.schema_version(),
    ) {
        BeginBuildDecision::Started(lease) => lease,
        other => panic!("unexpected begin decision: {other:?}"),
    };
    let store = service
        .open_store(SearchCorpusKind::ReferenceOccurrence)
        .await
        .unwrap_or_else(|error| panic!("open store: {error}"));
    let table_name =
        SearchPlaneService::table_name(SearchCorpusKind::ReferenceOccurrence, lease.epoch);
    store
        .replace_record_batches(
            table_name.as_str(),
            reference_occurrence_schema(),
            reference_occurrence_batches(hits).unwrap_or_else(|error| panic!("batches: {error}")),
        )
        .await
        .unwrap_or_else(|error| panic!("replace record batches: {error}"));
    store
        .write_vector_store_table_to_parquet_file(
            table_name.as_str(),
            service
                .local_epoch_parquet_path(SearchCorpusKind::ReferenceOccurrence, lease.epoch)
                .as_path(),
            ColumnarScanOptions::default(),
        )
        .await
        .unwrap_or_else(|error| panic!("export parquet: {error}"));
    service
        .coordinator()
        .publish_ready(&lease, hits.len() as u64, 1);
}
