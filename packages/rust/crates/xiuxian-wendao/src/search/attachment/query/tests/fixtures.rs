use std::path::PathBuf;

use crate::gateway::studio::types::{AttachmentSearchHit, StudioNavigationTarget};
use crate::search::attachment::schema::{
    attachment_batches, attachment_schema, search_text_column,
};
use crate::search::{
    BeginBuildDecision, SearchCorpusKind, SearchMaintenancePolicy, SearchManifestKeyspace,
    SearchPlaneService,
};

pub(super) fn fixture_service(temp_dir: &tempfile::TempDir) -> SearchPlaneService {
    SearchPlaneService::with_paths(
        PathBuf::from("/tmp/project"),
        temp_dir.path().join("search_plane"),
        SearchManifestKeyspace::new("xiuxian:test:attachment"),
        SearchMaintenancePolicy::default(),
    )
}

pub(super) fn sample_hit(
    name: &str,
    source_path: &str,
    attachment_path: &str,
    kind: &str,
) -> AttachmentSearchHit {
    AttachmentSearchHit {
        name: name.to_string(),
        path: source_path.to_string(),
        source_id: source_path.trim_end_matches(".md").to_string(),
        source_stem: "alpha".to_string(),
        source_title: "Alpha".to_string(),
        source_path: source_path.to_string(),
        attachment_id: format!("att://{source_path}/{attachment_path}"),
        attachment_path: attachment_path.to_string(),
        attachment_name: name.to_string(),
        attachment_ext: attachment_path
            .split('.')
            .next_back()
            .unwrap_or_default()
            .to_string(),
        kind: kind.to_string(),
        navigation_target: StudioNavigationTarget {
            path: source_path.to_string(),
            category: "doc".to_string(),
            project_name: None,
            root_label: None,
            line: None,
            line_end: None,
            column: None,
        },
        score: 0.0,
        vision_snippet: None,
    }
}

pub(super) async fn publish_attachment_hits(
    service: &SearchPlaneService,
    build_id: &str,
    hits: &[AttachmentSearchHit],
) {
    let lease = match service.coordinator().begin_build(
        SearchCorpusKind::Attachment,
        build_id,
        SearchCorpusKind::Attachment.schema_version(),
    ) {
        BeginBuildDecision::Started(lease) => lease,
        other => panic!("unexpected begin decision: {other:?}"),
    };
    let store = service
        .open_store(SearchCorpusKind::Attachment)
        .await
        .unwrap_or_else(|error| panic!("open store: {error}"));
    let table_name = SearchPlaneService::table_name(SearchCorpusKind::Attachment, lease.epoch);
    store
        .replace_record_batches(
            table_name.as_str(),
            attachment_schema(),
            attachment_batches(hits).unwrap_or_else(|error| panic!("batches: {error}")),
        )
        .await
        .unwrap_or_else(|error| panic!("replace record batches: {error}"));
    store
        .create_inverted_index(table_name.as_str(), search_text_column(), None)
        .await
        .unwrap_or_else(|error| panic!("create inverted index: {error}"));
    crate::search::attachment::build::export_attachment_epoch_parquet(service, lease.epoch)
        .await
        .unwrap_or_else(|error| panic!("export attachment parquet: {error}"));
    service
        .coordinator()
        .publish_ready(&lease, hits.len() as u64, 1);
}
