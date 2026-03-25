use std::collections::{BTreeMap, BTreeSet, HashSet};
use std::path::Path;

use tokio::runtime::Handle;
use xiuxian_vector::{ScalarIndexType, VectorStoreError};

use crate::gateway::studio::search::project_scope::project_metadata_for_path;
use crate::gateway::studio::types::{AttachmentSearchHit, StudioNavigationTarget, UiProjectConfig};
use crate::link_graph::LinkGraphAttachmentKind;
use crate::link_graph::parser::{is_supported_note, parse_note};
use crate::search_plane::{
    BeginBuildDecision, ProjectScannedFile, SearchBuildLease, SearchCorpusKind,
    SearchFileFingerprint, SearchPlaneService, delete_paths_from_table, fingerprint_note_projects,
    scan_note_project_files,
};

use super::schema::{
    attachment_batches, attachment_ext_column, attachment_schema, kind_column,
    projected_columns_with_hit_json, search_text_column, source_path_column,
};

const ATTACHMENT_EXTRACTOR_VERSION: u32 = 1;

#[derive(Debug, Clone)]
struct AttachmentBuildPlan {
    base_epoch: Option<u64>,
    file_fingerprints: BTreeMap<String, SearchFileFingerprint>,
    replaced_paths: BTreeSet<String>,
    changed_hits: Vec<AttachmentSearchHit>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct AttachmentWriteResult {
    row_count: u64,
    fragment_count: u64,
}

#[cfg(test)]
#[derive(Debug, thiserror::Error)]
pub(crate) enum AttachmentBuildError {
    #[error(transparent)]
    Storage(#[from] VectorStoreError),
}

pub(crate) fn ensure_attachment_index_started(
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
        SearchCorpusKind::Attachment,
        fingerprint,
        SearchCorpusKind::Attachment.schema_version(),
    );
    let BeginBuildDecision::Started(lease) = decision else {
        return;
    };

    let build_projects = projects.to_vec();
    let build_project_root = project_root.to_path_buf();
    let build_config_root = config_root.to_path_buf();
    let active_epoch = service.corpus_active_epoch(SearchCorpusKind::Attachment);
    let service = service.clone();

    if let Ok(handle) = Handle::try_current() {
        handle.spawn(async move {
            let previous_fingerprints = service
                .corpus_file_fingerprints(SearchCorpusKind::Attachment)
                .await;
            let build: Result<AttachmentBuildPlan, tokio::task::JoinError> =
                tokio::task::spawn_blocking(move || {
                    plan_attachment_build(
                        build_project_root.as_path(),
                        build_config_root.as_path(),
                        &build_projects,
                        active_epoch,
                        previous_fingerprints,
                    )
                })
                .await;

            match build {
                Ok(plan) => {
                    service.coordinator().update_progress(&lease, 0.3);
                    let write = write_attachment_epoch(&service, &lease, &plan).await;
                    if let Err(error) = write {
                        service
                            .coordinator()
                            .fail_build(&lease, format!("attachment epoch write failed: {error}"));
                        return;
                    }
                    let write = write.unwrap_or_else(|_| unreachable!());
                    let prewarm_columns = projected_columns_with_hit_json();
                    if let Err(error) = service
                        .prewarm_epoch_table(lease.corpus, lease.epoch, &prewarm_columns)
                        .await
                    {
                        service.coordinator().fail_build(
                            &lease,
                            format!("attachment epoch prewarm failed: {error}"),
                        );
                        return;
                    }
                    service.coordinator().update_progress(&lease, 0.9);
                    if service.publish_ready_and_maintain(
                        &lease,
                        write.row_count,
                        write.fragment_count,
                    ) {
                        service
                            .set_corpus_file_fingerprints(
                                SearchCorpusKind::Attachment,
                                &plan.file_fingerprints,
                            )
                            .await;
                    }
                    service.coordinator().update_progress(&lease, 1.0);
                }
                Err(error) => {
                    service.coordinator().fail_build(
                        &lease,
                        format!("attachment background build panicked: {error}"),
                    );
                }
            }
        });
    } else {
        service.coordinator().fail_build(
            &lease,
            "Tokio runtime unavailable for attachment index build",
        );
    }
}

fn plan_attachment_build(
    project_root: &Path,
    config_root: &Path,
    projects: &[UiProjectConfig],
    active_epoch: Option<u64>,
    previous_fingerprints: BTreeMap<String, SearchFileFingerprint>,
) -> AttachmentBuildPlan {
    let scanned_files = scan_note_project_files(project_root, config_root, projects);
    let file_fingerprints = scanned_files
        .iter()
        .map(|file| {
            (
                file.normalized_path.clone(),
                file.to_file_fingerprint(
                    ATTACHMENT_EXTRACTOR_VERSION,
                    SearchCorpusKind::Attachment.schema_version(),
                ),
            )
        })
        .collect::<BTreeMap<_, _>>();

    let can_incremental_reuse = active_epoch.is_some() && !previous_fingerprints.is_empty();
    if !can_incremental_reuse {
        return AttachmentBuildPlan {
            base_epoch: None,
            file_fingerprints,
            replaced_paths: BTreeSet::new(),
            changed_hits: build_attachment_hits_for_files(
                project_root,
                config_root,
                projects,
                &scanned_files,
            ),
        };
    }

    let changed_files = scanned_files
        .iter()
        .filter(|file| {
            previous_fingerprints.get(file.normalized_path.as_str())
                != file_fingerprints.get(file.normalized_path.as_str())
        })
        .cloned()
        .collect::<Vec<_>>();
    let mut replaced_paths = changed_files
        .iter()
        .map(|file| file.normalized_path.clone())
        .collect::<BTreeSet<_>>();
    for path in previous_fingerprints.keys() {
        if !file_fingerprints.contains_key(path) {
            replaced_paths.insert(path.clone());
        }
    }

    AttachmentBuildPlan {
        base_epoch: active_epoch,
        file_fingerprints,
        replaced_paths,
        changed_hits: build_attachment_hits_for_files(
            project_root,
            config_root,
            projects,
            &changed_files,
        ),
    }
}

async fn write_attachment_epoch(
    service: &SearchPlaneService,
    lease: &SearchBuildLease,
    plan: &AttachmentBuildPlan,
) -> Result<AttachmentWriteResult, VectorStoreError> {
    let store = service.open_store(SearchCorpusKind::Attachment).await?;
    let table_name = SearchPlaneService::table_name(SearchCorpusKind::Attachment, lease.epoch);
    let schema = attachment_schema();
    let changed_batches = attachment_batches(plan.changed_hits.as_slice())?;
    if let Some(base_epoch) = plan.base_epoch {
        let base_table_name =
            SearchPlaneService::table_name(SearchCorpusKind::Attachment, base_epoch);
        store
            .clone_table(base_table_name.as_str(), table_name.as_str(), true)
            .await?;
        delete_paths_from_table(
            &store,
            table_name.as_str(),
            source_path_column(),
            &plan.replaced_paths,
        )
        .await?;
        if !changed_batches.is_empty() {
            store
                .merge_insert_record_batches(
                    table_name.as_str(),
                    schema.clone(),
                    changed_batches,
                    &["id".to_string()],
                )
                .await?;
        }
    } else {
        store
            .replace_record_batches(table_name.as_str(), schema.clone(), changed_batches)
            .await?;
    }
    store
        .create_inverted_index(table_name.as_str(), search_text_column(), None)
        .await?;
    store
        .create_column_scalar_index(
            table_name.as_str(),
            attachment_ext_column(),
            None,
            ScalarIndexType::Bitmap,
        )
        .await?;
    store
        .create_column_scalar_index(
            table_name.as_str(),
            kind_column(),
            None,
            ScalarIndexType::Bitmap,
        )
        .await?;
    let table_info = store.get_table_info(table_name.as_str()).await?;
    Ok(AttachmentWriteResult {
        row_count: table_info.num_rows,
        fragment_count: u64::try_from(table_info.fragment_count).unwrap_or(u64::MAX),
    })
}

#[cfg(test)]
pub(crate) async fn publish_attachments_from_projects(
    service: &SearchPlaneService,
    project_root: &Path,
    config_root: &Path,
    projects: &[UiProjectConfig],
    fingerprint: &str,
) -> Result<(), AttachmentBuildError> {
    let lease = match service.coordinator().begin_build(
        SearchCorpusKind::Attachment,
        fingerprint,
        SearchCorpusKind::Attachment.schema_version(),
    ) {
        BeginBuildDecision::Started(lease) => lease,
        BeginBuildDecision::AlreadyReady(_) | BeginBuildDecision::AlreadyIndexing(_) => {
            return Ok(());
        }
    };
    let plan = plan_attachment_build(project_root, config_root, projects, None, BTreeMap::new());
    match write_attachment_epoch(service, &lease, &plan).await {
        Ok(write) => {
            let prewarm_columns = projected_columns_with_hit_json();
            service
                .prewarm_epoch_table(lease.corpus, lease.epoch, &prewarm_columns)
                .await?;
            service.publish_ready_and_maintain(&lease, write.row_count, write.fragment_count);
            Ok(())
        }
        Err(error) => {
            service
                .coordinator()
                .fail_build(&lease, format!("attachment epoch write failed: {error}"));
            Err(AttachmentBuildError::Storage(error))
        }
    }
}

fn build_attachment_hits_for_files(
    project_root: &Path,
    config_root: &Path,
    projects: &[UiProjectConfig],
    files: &[ProjectScannedFile],
) -> Vec<AttachmentSearchHit> {
    let mut hits = Vec::new();
    for file in files {
        hits.extend(build_attachment_hits_for_file(
            project_root,
            config_root,
            projects,
            file,
        ));
    }
    hits
}

fn build_attachment_hits_for_file(
    project_root: &Path,
    config_root: &Path,
    projects: &[UiProjectConfig],
    file: &ProjectScannedFile,
) -> Vec<AttachmentSearchHit> {
    if !is_supported_note(file.absolute_path.as_path()) {
        return Vec::new();
    }

    let Ok(content) = std::fs::read_to_string(file.absolute_path.as_path()) else {
        return Vec::new();
    };
    let Some(parsed) = parse_note(file.absolute_path.as_path(), project_root, &content) else {
        return Vec::new();
    };
    let metadata = project_metadata_for_path(
        project_root,
        config_root,
        projects,
        parsed.doc.path.as_str(),
    );
    attachment_hits_for_parsed_note(
        &parsed,
        metadata.project_name.as_deref(),
        metadata.root_label.as_deref(),
    )
}

fn attachment_hits_for_parsed_note(
    parsed: &crate::link_graph::parser::ParsedNote,
    project_name: Option<&str>,
    root_label: Option<&str>,
) -> Vec<AttachmentSearchHit> {
    let mut seen = HashSet::<String>::new();
    let mut hits = parsed
        .attachment_targets
        .iter()
        .filter(|attachment_path| seen.insert((*attachment_path).clone()))
        .map(|attachment_path| {
            let attachment_name = attachment_name(attachment_path);
            let attachment_ext = attachment_ext(attachment_path);
            AttachmentSearchHit {
                name: attachment_name.clone(),
                path: parsed.doc.path.clone(),
                source_id: parsed.doc.id.clone(),
                source_stem: parsed.doc.stem.clone(),
                source_title: parsed.doc.title.clone(),
                source_path: parsed.doc.path.clone(),
                attachment_id: format!("att://{}/{}", parsed.doc.id, attachment_path),
                attachment_path: attachment_path.clone(),
                attachment_name,
                attachment_ext: attachment_ext.clone(),
                kind: attachment_kind_label(LinkGraphAttachmentKind::from_extension(
                    attachment_ext.as_str(),
                ))
                .to_string(),
                navigation_target: StudioNavigationTarget {
                    path: parsed.doc.path.clone(),
                    category: "doc".to_string(),
                    project_name: project_name.map(ToString::to_string),
                    root_label: root_label.map(ToString::to_string),
                    line: None,
                    line_end: None,
                    column: None,
                },
                score: 0.0,
                vision_snippet: None,
            }
        })
        .collect::<Vec<_>>();
    hits.sort_by(|left, right| {
        left.attachment_path
            .cmp(&right.attachment_path)
            .then(left.source_path.cmp(&right.source_path))
    });
    hits
}

fn attachment_name(path: &str) -> String {
    Path::new(path)
        .file_name()
        .and_then(|value| value.to_str())
        .map_or_else(|| path.to_string(), ToString::to_string)
}

fn attachment_ext(path: &str) -> String {
    Path::new(path)
        .extension()
        .and_then(|value| value.to_str())
        .map(|value| value.trim().trim_start_matches('.').to_ascii_lowercase())
        .unwrap_or_default()
}

pub(crate) fn attachment_kind_label(kind: LinkGraphAttachmentKind) -> &'static str {
    match kind {
        LinkGraphAttachmentKind::Image => "image",
        LinkGraphAttachmentKind::Pdf => "pdf",
        LinkGraphAttachmentKind::Gpg => "gpg",
        LinkGraphAttachmentKind::Document => "document",
        LinkGraphAttachmentKind::Archive => "archive",
        LinkGraphAttachmentKind::Audio => "audio",
        LinkGraphAttachmentKind::Video => "video",
        LinkGraphAttachmentKind::Other => "other",
    }
}

fn fingerprint_projects(
    project_root: &Path,
    config_root: &Path,
    projects: &[UiProjectConfig],
) -> String {
    fingerprint_note_projects(project_root, config_root, projects)
}

#[cfg(test)]
mod tests {
    use std::collections::{BTreeMap, BTreeSet};
    use std::time::Duration;

    use super::plan_attachment_build;
    use crate::gateway::studio::types::UiProjectConfig;
    use crate::link_graph::LinkGraphAttachmentKind;
    use crate::search_plane::attachment::search_attachment_hits;
    use crate::search_plane::cache::SearchPlaneCache;
    use crate::search_plane::{
        SearchCorpusKind, SearchMaintenancePolicy, SearchManifestKeyspace, SearchPlanePhase,
        SearchPlaneService,
    };

    #[test]
    fn plan_attachment_build_only_reparses_changed_notes() {
        let temp_dir = tempfile::tempdir().unwrap_or_else(|error| panic!("temp dir: {error}"));
        let project_root = temp_dir.path();
        std::fs::create_dir_all(project_root.join("docs"))
            .unwrap_or_else(|error| panic!("create docs: {error}"));
        std::fs::write(
            project_root.join("docs/alpha.md"),
            "# Alpha\n\n![Topology](assets/topology.png)\n",
        )
        .unwrap_or_else(|error| panic!("write alpha note: {error}"));
        std::fs::write(
            project_root.join("docs/beta.md"),
            "# Beta\n\n![Avatar](images/avatar.jpg)\n",
        )
        .unwrap_or_else(|error| panic!("write beta note: {error}"));
        let projects = vec![UiProjectConfig {
            name: "kernel".to_string(),
            root: ".".to_string(),
            dirs: vec!["docs".to_string()],
        }];

        let first =
            plan_attachment_build(project_root, project_root, &projects, None, BTreeMap::new());
        assert_eq!(first.base_epoch, None);
        assert!(
            first
                .changed_hits
                .iter()
                .any(|hit| hit.source_path == "docs/alpha.md"
                    && hit.attachment_name == "topology.png")
        );
        assert!(
            first
                .changed_hits
                .iter()
                .any(|hit| hit.source_path == "docs/beta.md" && hit.attachment_name == "avatar.jpg")
        );

        std::thread::sleep(Duration::from_millis(5));
        std::fs::write(
            project_root.join("docs/alpha.md"),
            "# Alpha\n\n![Diagram](assets/diagram.png)\n",
        )
        .unwrap_or_else(|error| panic!("rewrite alpha note: {error}"));

        let second = plan_attachment_build(
            project_root,
            project_root,
            &projects,
            Some(7),
            first.file_fingerprints.clone(),
        );
        assert_eq!(second.base_epoch, Some(7));
        assert_eq!(
            second.replaced_paths,
            BTreeSet::from(["docs/alpha.md".to_string()])
        );
        assert!(
            second
                .changed_hits
                .iter()
                .all(|hit| hit.source_path == "docs/alpha.md")
        );
        assert!(
            second
                .changed_hits
                .iter()
                .any(|hit| hit.attachment_name == "diagram.png")
        );
        assert!(
            second
                .changed_hits
                .iter()
                .all(|hit| hit.attachment_name != "avatar.jpg"),
            "unchanged note attachments must not be reparsed into the changed set"
        );
    }

    #[tokio::test]
    async fn attachment_incremental_refresh_reuses_unchanged_rows() {
        let temp_dir = tempfile::tempdir().unwrap_or_else(|error| panic!("temp dir: {error}"));
        let project_root = temp_dir.path().join("workspace");
        let storage_root = temp_dir.path().join("search_plane");
        std::fs::create_dir_all(project_root.join("docs"))
            .unwrap_or_else(|error| panic!("create docs: {error}"));
        std::fs::write(
            project_root.join("docs/alpha.md"),
            "# Alpha\n\n![Topology](assets/topology.png)\n",
        )
        .unwrap_or_else(|error| panic!("write alpha note: {error}"));
        std::fs::write(
            project_root.join("docs/beta.md"),
            "# Beta\n\n![Avatar](images/avatar.jpg)\n",
        )
        .unwrap_or_else(|error| panic!("write beta note: {error}"));
        let projects = vec![UiProjectConfig {
            name: "kernel".to_string(),
            root: ".".to_string(),
            dirs: vec!["docs".to_string()],
        }];
        let keyspace =
            SearchManifestKeyspace::new("xiuxian:test:search_plane:attachment-incremental");
        let cache = SearchPlaneCache::for_tests(keyspace.clone());
        let service = SearchPlaneService::with_runtime(
            project_root.clone(),
            storage_root,
            keyspace,
            SearchMaintenancePolicy::default(),
            cache,
        );

        super::ensure_attachment_index_started(
            &service,
            project_root.as_path(),
            project_root.as_path(),
            &projects,
        );
        wait_for_attachment_ready(&service, None).await;

        let initial_avatar = search_attachment_hits(&service, "avatar", 10, &[], &[], false)
            .await
            .unwrap_or_else(|error| panic!("query avatar: {error}"));
        assert_eq!(initial_avatar.len(), 1);
        let initial_topology = search_attachment_hits(&service, "topology", 10, &[], &[], false)
            .await
            .unwrap_or_else(|error| panic!("query topology: {error}"));
        assert_eq!(initial_topology.len(), 1);

        std::fs::write(
            project_root.join("docs/alpha.md"),
            "# Alpha\n\n![Diagram](assets/diagram.png)\n",
        )
        .unwrap_or_else(|error| panic!("rewrite alpha note: {error}"));
        super::ensure_attachment_index_started(
            &service,
            project_root.as_path(),
            project_root.as_path(),
            &projects,
        );
        wait_for_attachment_ready(&service, Some(1)).await;

        let avatar = search_attachment_hits(&service, "avatar", 10, &[], &[], false)
            .await
            .unwrap_or_else(|error| panic!("query avatar after refresh: {error}"));
        assert_eq!(avatar.len(), 1);
        let diagram = search_attachment_hits(&service, "diagram", 10, &[], &[], false)
            .await
            .unwrap_or_else(|error| panic!("query diagram after refresh: {error}"));
        assert_eq!(diagram.len(), 1);
        assert_eq!(diagram[0].kind, "image");
        let topology = search_attachment_hits(
            &service,
            "topology",
            10,
            &[],
            &[LinkGraphAttachmentKind::Image],
            false,
        )
        .await
        .unwrap_or_else(|error| panic!("query topology after refresh: {error}"));
        assert!(topology.is_empty());
    }

    async fn wait_for_attachment_ready(service: &SearchPlaneService, previous_epoch: Option<u64>) {
        for _ in 0..100 {
            let status = service
                .coordinator()
                .status_for(SearchCorpusKind::Attachment);
            if status.phase == SearchPlanePhase::Ready
                && status.active_epoch.is_some()
                && previous_epoch
                    .is_none_or(|epoch| status.active_epoch.unwrap_or_default() > epoch)
            {
                return;
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
        panic!("attachment build did not reach ready state");
    }
}
