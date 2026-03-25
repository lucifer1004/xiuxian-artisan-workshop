use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use tokio::runtime::Handle;
use xiuxian_vector::VectorStoreError;

use crate::gateway::studio::search::project_scope::{
    SearchProjectMetadata, project_metadata_for_path, resolve_project_root_path,
};
use crate::gateway::studio::types::{SearchHit, StudioNavigationTarget, UiProjectConfig};
use crate::link_graph::parser::{ParsedNote, ParsedSection, is_supported_note, parse_note};
use crate::search_plane::{
    BeginBuildDecision, ProjectScannedFile, SearchBuildLease, SearchCorpusKind,
    SearchFileFingerprint, SearchPlaneService, delete_paths_from_table, fingerprint_note_projects,
    scan_note_project_files,
};

use super::schema::{
    KnowledgeSectionRow, knowledge_section_batches, knowledge_section_schema, path_column,
    projected_columns, search_text_column,
};

const KNOWLEDGE_SECTION_EXTRACTOR_VERSION: u32 = 1;

#[derive(Debug, Clone)]
struct KnowledgeSectionBuildPlan {
    base_epoch: Option<u64>,
    file_fingerprints: BTreeMap<String, SearchFileFingerprint>,
    replaced_paths: BTreeSet<String>,
    changed_rows: Vec<KnowledgeSectionRow>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct KnowledgeSectionWriteResult {
    row_count: u64,
    fragment_count: u64,
}

#[cfg(test)]
#[derive(Debug, thiserror::Error)]
pub(crate) enum KnowledgeSectionBuildError {
    #[error(transparent)]
    Storage(#[from] VectorStoreError),
}

pub(crate) fn ensure_knowledge_section_index_started(
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
        SearchCorpusKind::KnowledgeSection,
        fingerprint,
        SearchCorpusKind::KnowledgeSection.schema_version(),
    );
    let BeginBuildDecision::Started(lease) = decision else {
        return;
    };

    let build_projects = projects.to_vec();
    let build_project_root = project_root.to_path_buf();
    let build_config_root = config_root.to_path_buf();
    let active_epoch = service.corpus_active_epoch(SearchCorpusKind::KnowledgeSection);
    let service = service.clone();

    if let Ok(handle) = Handle::try_current() {
        handle.spawn(async move {
            let previous_fingerprints = service
                .corpus_file_fingerprints(SearchCorpusKind::KnowledgeSection)
                .await;
            let build: Result<KnowledgeSectionBuildPlan, tokio::task::JoinError> =
                tokio::task::spawn_blocking(move || {
                    plan_knowledge_section_build(
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
                    let write = write_knowledge_section_epoch(&service, &lease, &plan).await;
                    if let Err(error) = write {
                        service.coordinator().fail_build(
                            &lease,
                            format!("knowledge section epoch write failed: {error}"),
                        );
                        return;
                    }
                    let write = write.unwrap_or_else(|_| unreachable!());
                    let prewarm_columns = projected_columns();
                    if let Err(error) = service
                        .prewarm_epoch_table(lease.corpus, lease.epoch, &prewarm_columns)
                        .await
                    {
                        service.coordinator().fail_build(
                            &lease,
                            format!("knowledge section epoch prewarm failed: {error}"),
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
                                SearchCorpusKind::KnowledgeSection,
                                &plan.file_fingerprints,
                            )
                            .await;
                    }
                    service.coordinator().update_progress(&lease, 1.0);
                }
                Err(error) => {
                    service.coordinator().fail_build(
                        &lease,
                        format!("knowledge section background build panicked: {error}"),
                    );
                }
            }
        });
    } else {
        service.coordinator().fail_build(
            &lease,
            "Tokio runtime unavailable for knowledge section build",
        );
    }
}

fn plan_knowledge_section_build(
    project_root: &Path,
    config_root: &Path,
    projects: &[UiProjectConfig],
    active_epoch: Option<u64>,
    previous_fingerprints: BTreeMap<String, SearchFileFingerprint>,
) -> KnowledgeSectionBuildPlan {
    let scanned_files = scan_note_project_files(project_root, config_root, projects);
    let file_fingerprints = scanned_files
        .iter()
        .map(|file| {
            (
                file.normalized_path.clone(),
                file.to_file_fingerprint(
                    KNOWLEDGE_SECTION_EXTRACTOR_VERSION,
                    SearchCorpusKind::KnowledgeSection.schema_version(),
                ),
            )
        })
        .collect::<BTreeMap<_, _>>();

    let can_incremental_reuse = active_epoch.is_some() && !previous_fingerprints.is_empty();
    if !can_incremental_reuse {
        return KnowledgeSectionBuildPlan {
            base_epoch: None,
            file_fingerprints,
            replaced_paths: BTreeSet::new(),
            changed_rows: build_knowledge_section_rows_for_files(
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

    KnowledgeSectionBuildPlan {
        base_epoch: active_epoch,
        file_fingerprints,
        replaced_paths,
        changed_rows: build_knowledge_section_rows_for_files(
            project_root,
            config_root,
            projects,
            &changed_files,
        ),
    }
}

async fn write_knowledge_section_epoch(
    service: &SearchPlaneService,
    lease: &SearchBuildLease,
    plan: &KnowledgeSectionBuildPlan,
) -> Result<KnowledgeSectionWriteResult, VectorStoreError> {
    let store = service
        .open_store(SearchCorpusKind::KnowledgeSection)
        .await?;
    let table_name =
        SearchPlaneService::table_name(SearchCorpusKind::KnowledgeSection, lease.epoch);
    let schema = knowledge_section_schema();
    let changed_batches = knowledge_section_batches(plan.changed_rows.as_slice())?;
    if let Some(base_epoch) = plan.base_epoch {
        let base_table_name =
            SearchPlaneService::table_name(SearchCorpusKind::KnowledgeSection, base_epoch);
        store
            .clone_table(base_table_name.as_str(), table_name.as_str(), true)
            .await?;
        delete_paths_from_table(
            &store,
            table_name.as_str(),
            path_column(),
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
    let table_info = store.get_table_info(table_name.as_str()).await?;
    Ok(KnowledgeSectionWriteResult {
        row_count: table_info.num_rows,
        fragment_count: u64::try_from(table_info.fragment_count).unwrap_or(u64::MAX),
    })
}

#[cfg(test)]
pub(crate) async fn publish_knowledge_sections_from_projects(
    service: &SearchPlaneService,
    project_root: &Path,
    config_root: &Path,
    projects: &[UiProjectConfig],
    fingerprint: &str,
) -> Result<(), KnowledgeSectionBuildError> {
    let lease = match service.coordinator().begin_build(
        SearchCorpusKind::KnowledgeSection,
        fingerprint,
        SearchCorpusKind::KnowledgeSection.schema_version(),
    ) {
        BeginBuildDecision::Started(lease) => lease,
        BeginBuildDecision::AlreadyReady(_) | BeginBuildDecision::AlreadyIndexing(_) => {
            return Ok(());
        }
    };
    let plan =
        plan_knowledge_section_build(project_root, config_root, projects, None, BTreeMap::new());
    match write_knowledge_section_epoch(service, &lease, &plan).await {
        Ok(write) => {
            let prewarm_columns = projected_columns();
            service
                .prewarm_epoch_table(lease.corpus, lease.epoch, &prewarm_columns)
                .await?;
            service.publish_ready_and_maintain(&lease, write.row_count, write.fragment_count);
            Ok(())
        }
        Err(error) => {
            service.coordinator().fail_build(
                &lease,
                format!("knowledge section epoch write failed: {error}"),
            );
            Err(KnowledgeSectionBuildError::Storage(error))
        }
    }
}

fn build_knowledge_section_rows_for_files(
    project_root: &Path,
    config_root: &Path,
    projects: &[UiProjectConfig],
    files: &[ProjectScannedFile],
) -> Vec<KnowledgeSectionRow> {
    let mut rows = Vec::new();
    for file in files {
        rows.extend(build_knowledge_section_rows_for_file(
            project_root,
            config_root,
            projects,
            file,
        ));
    }
    rows
}

fn build_knowledge_section_rows_for_file(
    project_root: &Path,
    config_root: &Path,
    projects: &[UiProjectConfig],
    file: &ProjectScannedFile,
) -> Vec<KnowledgeSectionRow> {
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
    knowledge_rows_for_note(project_root, config_root, projects, &parsed, &metadata)
}

fn knowledge_rows_for_note(
    project_root: &Path,
    config_root: &Path,
    projects: &[UiProjectConfig],
    parsed: &ParsedNote,
    metadata: &SearchProjectMetadata,
) -> Vec<KnowledgeSectionRow> {
    let display_path = studio_display_path(
        project_root,
        config_root,
        projects,
        metadata,
        parsed.doc.path.as_str(),
    );
    let hierarchy = Some(
        display_path
            .split('/')
            .filter(|segment| !segment.is_empty())
            .map(str::to_string)
            .collect::<Vec<_>>(),
    );
    let navigation_target = Some(StudioNavigationTarget {
        path: parsed.doc.path.clone(),
        category: "doc".to_string(),
        project_name: metadata.project_name.clone(),
        root_label: metadata.root_label.clone(),
        line: None,
        line_end: None,
        column: None,
    });
    let mut tags = parsed.doc.tags.clone();
    if let Some(doc_type) = parsed.doc.doc_type.as_deref()
        && !tags.iter().any(|tag| tag == doc_type)
    {
        tags.push(doc_type.to_string());
    }

    let doc_hit = SearchHit {
        stem: parsed.doc.stem.clone(),
        title: Some(parsed.doc.title.clone()),
        path: display_path.clone(),
        doc_type: parsed.doc.doc_type.clone(),
        tags: tags.clone(),
        score: 0.0,
        best_section: None,
        match_reason: Some("knowledge_section_search".to_string()),
        hierarchical_uri: None,
        hierarchy: hierarchy.clone(),
        saliency_score: None,
        audit_status: None,
        verification_state: None,
        implicit_backlinks: None,
        implicit_backlink_items: None,
        navigation_target: navigation_target.clone(),
    };
    let doc_search_text = normalize_search_text(
        std::iter::once(parsed.doc.title.as_str())
            .chain(std::iter::once(parsed.doc.stem.as_str()))
            .chain(parsed.doc.doc_type.iter().map(String::as_str))
            .chain(tags.iter().map(String::as_str))
            .chain(parsed.sections.iter().flat_map(|section| {
                [
                    section.heading_title.as_str(),
                    section.heading_path.as_str(),
                    section.section_text.as_str(),
                ]
            })),
    );
    let mut rows = vec![doc_row_for_note(&display_path, &doc_hit, doc_search_text)];
    rows.extend(parsed.sections.iter().map(|section| {
        section_row_for_note(
            &display_path,
            parsed,
            section,
            &tags,
            hierarchy.clone(),
            navigation_target.clone(),
        )
    }));

    rows
}

fn doc_row_for_note(
    display_path: &str,
    doc_hit: &SearchHit,
    doc_search_text: String,
) -> KnowledgeSectionRow {
    KnowledgeSectionRow {
        id: format!("{display_path}:_doc"),
        path: display_path.to_string(),
        stem: doc_hit.stem.clone(),
        title: doc_hit.title.clone(),
        best_section: None,
        search_text: doc_search_text,
        hit_json: serialize_hit(doc_hit),
    }
}

fn section_row_for_note(
    display_path: &str,
    parsed: &ParsedNote,
    section: &ParsedSection,
    tags: &[String],
    hierarchy: Option<Vec<String>>,
    navigation_target: Option<StudioNavigationTarget>,
) -> KnowledgeSectionRow {
    let hit = SearchHit {
        stem: parsed.doc.stem.clone(),
        title: Some(parsed.doc.title.clone()),
        path: display_path.to_string(),
        doc_type: parsed.doc.doc_type.clone(),
        tags: tags.to_vec(),
        score: 0.0,
        best_section: Some(section.heading_path.clone()),
        match_reason: Some("knowledge_section_search".to_string()),
        hierarchical_uri: None,
        hierarchy,
        saliency_score: None,
        audit_status: None,
        verification_state: None,
        implicit_backlinks: None,
        implicit_backlink_items: None,
        navigation_target,
    };
    KnowledgeSectionRow {
        id: format!("{display_path}:{}", section.heading_path),
        path: display_path.to_string(),
        stem: parsed.doc.stem.clone(),
        title: Some(parsed.doc.title.clone()),
        best_section: Some(section.heading_path.clone()),
        search_text: normalize_search_text([
            parsed.doc.title.as_str(),
            parsed.doc.stem.as_str(),
            section.heading_title.as_str(),
            section.heading_path.as_str(),
            section.section_text.as_str(),
        ]),
        hit_json: serialize_hit(&hit),
    }
}

fn serialize_hit(hit: &SearchHit) -> String {
    serde_json::to_string(hit).unwrap_or_else(|error| {
        panic!("serialize knowledge hit should succeed: {error}");
    })
}

fn normalize_search_text<'a>(segments: impl IntoIterator<Item = &'a str>) -> String {
    let mut normalized = String::new();
    for segment in segments {
        let trimmed = segment.trim();
        if trimmed.is_empty() {
            continue;
        }
        if !normalized.is_empty() {
            normalized.push(' ');
        }
        normalized.push_str(trimmed);
        if normalized.len() >= 16 * 1024 {
            normalized.truncate(16 * 1024);
            break;
        }
    }
    normalized
}

fn studio_display_path(
    project_root: &Path,
    config_root: &Path,
    projects: &[UiProjectConfig],
    metadata: &SearchProjectMetadata,
    path: &str,
) -> String {
    let normalized = path.replace('\\', "/");
    if projects.len() > 1
        && let Some(project_name) = metadata.project_name.as_deref()
    {
        let relative_to_project = projects
            .iter()
            .find(|project| project.name == project_name)
            .and_then(|project| resolve_project_root_path(config_root, project.root.as_str()))
            .and_then(|project_root_path| {
                let absolute_path = if Path::new(path).is_absolute() {
                    Path::new(path).to_path_buf()
                } else {
                    project_root.join(path)
                };
                absolute_path
                    .strip_prefix(project_root_path)
                    .ok()
                    .map(|relative| relative.to_string_lossy().replace('\\', "/"))
            })
            .filter(|relative| !relative.is_empty())
            .unwrap_or_else(|| normalized.clone());

        if !relative_to_project.starts_with(&format!("{project_name}/")) {
            return format!("{project_name}/{relative_to_project}");
        }
    }

    normalized
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

    use super::{fingerprint_projects, plan_knowledge_section_build};
    use crate::gateway::studio::types::UiProjectConfig;
    use crate::search_plane::cache::SearchPlaneCache;
    use crate::search_plane::knowledge_section::search_knowledge_sections;
    use crate::search_plane::{
        SearchCorpusKind, SearchMaintenancePolicy, SearchManifestKeyspace, SearchPlanePhase,
        SearchPlaneService,
    };

    #[test]
    fn plan_knowledge_section_build_only_reparses_changed_notes() {
        let temp_dir = tempfile::tempdir().unwrap_or_else(|error| panic!("temp dir: {error}"));
        let project_root = temp_dir.path();
        std::fs::create_dir_all(project_root.join("notes"))
            .unwrap_or_else(|error| panic!("create notes: {error}"));
        std::fs::write(
            project_root.join("notes/alpha.md"),
            "# Alpha\n\nAlpha body.\n\n## Overview\n\nAlpha section.\n",
        )
        .unwrap_or_else(|error| panic!("write alpha note: {error}"));
        std::fs::write(
            project_root.join("notes/gamma.md"),
            "# Gamma\n\nGamma body.\n\n## Overview\n\nGamma section.\n",
        )
        .unwrap_or_else(|error| panic!("write gamma note: {error}"));
        let projects = vec![UiProjectConfig {
            name: "notes".to_string(),
            root: ".".to_string(),
            dirs: vec![".".to_string()],
        }];

        let first = plan_knowledge_section_build(
            project_root,
            project_root,
            &projects,
            None,
            BTreeMap::new(),
        );
        assert_eq!(first.base_epoch, None);
        assert!(
            first
                .changed_rows
                .iter()
                .any(|row| row.path == "notes/alpha.md")
        );
        assert!(
            first
                .changed_rows
                .iter()
                .any(|row| row.path == "notes/gamma.md")
        );

        std::thread::sleep(Duration::from_millis(5));
        std::fs::write(
            project_root.join("notes/alpha.md"),
            "# Beta\n\nBeta body.\n\n## Overview\n\nBeta section.\n",
        )
        .unwrap_or_else(|error| panic!("rewrite alpha note: {error}"));

        let second = plan_knowledge_section_build(
            project_root,
            project_root,
            &projects,
            Some(7),
            first.file_fingerprints.clone(),
        );
        assert_eq!(second.base_epoch, Some(7));
        assert_eq!(
            second.replaced_paths,
            BTreeSet::from(["notes/alpha.md".to_string()])
        );
        assert!(
            second
                .changed_rows
                .iter()
                .all(|row| row.path == "notes/alpha.md")
        );
        assert!(
            second
                .changed_rows
                .iter()
                .any(|row| row.search_text.contains("Beta"))
        );
        assert!(
            second
                .changed_rows
                .iter()
                .all(|row| !row.path.contains("gamma")),
            "unchanged note rows must not be reparsed into the changed set"
        );
    }

    #[tokio::test]
    async fn knowledge_section_incremental_refresh_reuses_unchanged_rows() {
        let temp_dir = tempfile::tempdir().unwrap_or_else(|error| panic!("temp dir: {error}"));
        let project_root = temp_dir.path().join("workspace");
        let storage_root = temp_dir.path().join("search_plane");
        std::fs::create_dir_all(project_root.join("notes"))
            .unwrap_or_else(|error| panic!("create notes: {error}"));
        std::fs::write(
            project_root.join("notes/alpha.md"),
            "# Alpha\n\nAlpha body.\n\n## Overview\n\nAlpha section.\n",
        )
        .unwrap_or_else(|error| panic!("write alpha note: {error}"));
        std::fs::write(
            project_root.join("notes/gamma.md"),
            "# Gamma\n\nGamma body.\n\n## Overview\n\nGamma section.\n",
        )
        .unwrap_or_else(|error| panic!("write gamma note: {error}"));
        let projects = vec![UiProjectConfig {
            name: "notes".to_string(),
            root: ".".to_string(),
            dirs: vec![".".to_string()],
        }];
        let keyspace =
            SearchManifestKeyspace::new("xiuxian:test:search_plane:knowledge-section-incremental");
        let cache = SearchPlaneCache::for_tests(keyspace.clone());
        let service = SearchPlaneService::with_runtime(
            project_root.clone(),
            storage_root,
            keyspace,
            SearchMaintenancePolicy::default(),
            cache,
        );

        super::ensure_knowledge_section_index_started(
            &service,
            project_root.as_path(),
            project_root.as_path(),
            &projects,
        );
        wait_for_knowledge_section_ready(&service, None).await;

        let initial_gamma = search_knowledge_sections(&service, "Gamma body", 10)
            .await
            .unwrap_or_else(|error| panic!("query gamma: {error}"));
        assert_eq!(initial_gamma.len(), 1);
        let initial_alpha = search_knowledge_sections(&service, "Alpha body", 10)
            .await
            .unwrap_or_else(|error| panic!("query alpha: {error}"));
        assert_eq!(initial_alpha.len(), 1);

        std::fs::write(
            project_root.join("notes/alpha.md"),
            "# Beta\n\nBeta body.\n\n## Overview\n\nBeta section.\n",
        )
        .unwrap_or_else(|error| panic!("rewrite alpha note: {error}"));
        super::ensure_knowledge_section_index_started(
            &service,
            project_root.as_path(),
            project_root.as_path(),
            &projects,
        );
        wait_for_knowledge_section_ready(&service, Some(1)).await;

        let gamma = search_knowledge_sections(&service, "Gamma body", 10)
            .await
            .unwrap_or_else(|error| panic!("query gamma after refresh: {error}"));
        assert_eq!(gamma.len(), 1);
        let beta = search_knowledge_sections(&service, "Beta body", 10)
            .await
            .unwrap_or_else(|error| panic!("query beta after refresh: {error}"));
        assert_eq!(beta.len(), 1);
        let alpha = search_knowledge_sections(&service, "Alpha body", 10)
            .await
            .unwrap_or_else(|error| panic!("query alpha after refresh: {error}"));
        assert!(alpha.is_empty());
    }

    async fn wait_for_knowledge_section_ready(
        service: &SearchPlaneService,
        previous_epoch: Option<u64>,
    ) {
        for _ in 0..100 {
            let status = service
                .coordinator()
                .status_for(SearchCorpusKind::KnowledgeSection);
            if status.phase == SearchPlanePhase::Ready
                && status.active_epoch.is_some()
                && previous_epoch
                    .is_none_or(|epoch| status.active_epoch.unwrap_or_default() > epoch)
            {
                return;
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
        panic!("knowledge section build did not reach ready state");
    }

    #[test]
    fn fingerprint_projects_changes_when_scanned_note_metadata_changes() {
        let temp_dir = tempfile::tempdir().unwrap_or_else(|error| panic!("temp dir: {error}"));
        let project_root = temp_dir.path();
        std::fs::create_dir_all(project_root.join("notes"))
            .unwrap_or_else(|error| panic!("create notes: {error}"));
        std::fs::create_dir_all(project_root.join("node_modules/pkg"))
            .unwrap_or_else(|error| panic!("create skipped dir: {error}"));
        std::fs::write(
            project_root.join("notes/alpha.md"),
            "# Alpha\n\nAlpha body.\n",
        )
        .unwrap_or_else(|error| panic!("write note: {error}"));
        std::fs::write(
            project_root.join("node_modules/pkg/ignored.md"),
            "# Ignored\n",
        )
        .unwrap_or_else(|error| panic!("write skipped file: {error}"));

        let projects = vec![UiProjectConfig {
            name: "notes".to_string(),
            root: ".".to_string(),
            dirs: vec![".".to_string()],
        }];

        let first = fingerprint_projects(project_root, project_root, &projects);
        std::fs::write(
            project_root.join("node_modules/pkg/ignored.md"),
            "# Still Ignored\n",
        )
        .unwrap_or_else(|error| panic!("rewrite skipped note: {error}"));
        let after_skipped_change = fingerprint_projects(project_root, project_root, &projects);
        assert_eq!(first, after_skipped_change);

        std::fs::write(
            project_root.join("notes/alpha.md"),
            "# Beta\n\nBeta body.\n",
        )
        .unwrap_or_else(|error| panic!("rewrite note: {error}"));
        let second = fingerprint_projects(project_root, project_root, &projects);
        assert_ne!(first, second);
    }
}
