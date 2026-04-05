use std::path::PathBuf;

use crate::gateway::studio::types::{AstSearchHit, StudioNavigationTarget};
use crate::search_plane::local_symbol::schema::{local_symbol_batches, local_symbol_schema};
use crate::search_plane::{
    BeginBuildDecision, SearchCorpusKind, SearchMaintenancePolicy, SearchManifestKeyspace,
    SearchPlaneService,
};
use xiuxian_vector::ColumnarScanOptions;

pub(super) fn fixture_service(temp_dir: &tempfile::TempDir) -> SearchPlaneService {
    SearchPlaneService::with_paths(
        PathBuf::from("/tmp/project"),
        temp_dir.path().join("search_plane"),
        SearchManifestKeyspace::new("xiuxian:test:local_symbol"),
        SearchMaintenancePolicy::default(),
    )
}

pub(super) fn sample_hit(name: &str, path: &str, line_start: usize) -> AstSearchHit {
    AstSearchHit {
        name: name.to_string(),
        signature: format!("fn {name}()"),
        path: path.to_string(),
        language: "rust".to_string(),
        crate_name: "kernel".to_string(),
        project_name: None,
        root_label: None,
        node_kind: None,
        owner_title: None,
        navigation_target: StudioNavigationTarget {
            path: path.to_string(),
            category: "symbol".to_string(),
            project_name: None,
            root_label: None,
            line: Some(line_start),
            line_end: Some(line_start),
            column: Some(1),
        },
        line_start,
        line_end: line_start,
        score: 0.0,
    }
}

pub(super) fn sample_markdown_hit(
    name: &str,
    node_kind: Option<&str>,
    owner_title: Option<&str>,
) -> AstSearchHit {
    AstSearchHit {
        name: name.to_string(),
        signature: format!("## {name}"),
        path: "docs/alpha.md".to_string(),
        language: "markdown".to_string(),
        crate_name: "docs".to_string(),
        project_name: None,
        root_label: None,
        node_kind: node_kind.map(ToOwned::to_owned),
        owner_title: owner_title.map(ToOwned::to_owned),
        navigation_target: StudioNavigationTarget {
            path: "docs/alpha.md".to_string(),
            category: "symbol".to_string(),
            project_name: None,
            root_label: None,
            line: Some(1),
            line_end: Some(1),
            column: Some(1),
        },
        line_start: 1,
        line_end: 1,
        score: 0.0,
    }
}

pub(super) async fn publish_local_symbol_hits(
    service: &SearchPlaneService,
    build_id: &str,
    hits: &[AstSearchHit],
) {
    let lease = match service.coordinator().begin_build(
        SearchCorpusKind::LocalSymbol,
        build_id,
        SearchCorpusKind::LocalSymbol.schema_version(),
    ) {
        BeginBuildDecision::Started(lease) => lease,
        other => panic!("unexpected begin decision: {other:?}"),
    };
    let store = service
        .open_store(SearchCorpusKind::LocalSymbol)
        .await
        .unwrap_or_else(|error| panic!("open store: {error}"));
    let table_name = SearchPlaneService::table_name(SearchCorpusKind::LocalSymbol, lease.epoch);
    store
        .replace_record_batches(
            table_name.as_str(),
            local_symbol_schema(),
            local_symbol_batches(hits).unwrap_or_else(|error| panic!("batches: {error}")),
        )
        .await
        .unwrap_or_else(|error| panic!("replace record batches: {error}"));
    store
        .write_vector_store_table_to_parquet_file(
            table_name.as_str(),
            service
                .local_epoch_parquet_path(SearchCorpusKind::LocalSymbol, lease.epoch)
                .as_path(),
            ColumnarScanOptions::default(),
        )
        .await
        .unwrap_or_else(|error| panic!("export parquet: {error}"));
    service
        .coordinator()
        .publish_ready(&lease, hits.len() as u64, 1);
}
