use std::collections::HashSet;
use std::path::{Path, PathBuf};

use tokio::runtime::Handle;
use walkdir::{DirEntry, WalkDir};
use xiuxian_vector::VectorStoreError;

use crate::gateway::studio::search::project_scope::{
    SearchProjectMetadata, configured_project_scan_roots, project_metadata_for_path,
    resolve_project_root_path,
};
use crate::gateway::studio::types::{SearchHit, StudioNavigationTarget, UiProjectConfig};
use crate::link_graph::parser::{ParsedNote, ParsedSection, is_supported_note, parse_note};
use crate::search_plane::{
    BeginBuildDecision, SearchBuildLease, SearchCorpusKind, SearchPlaneService,
};

use super::schema::{
    KnowledgeSectionRow, knowledge_section_batches, knowledge_section_schema, search_text_column,
};

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
    let service = service.clone();

    if let Ok(handle) = Handle::try_current() {
        handle.spawn(async move {
            let build: Result<Vec<KnowledgeSectionRow>, tokio::task::JoinError> =
                tokio::task::spawn_blocking(move || {
                    build_knowledge_section_rows(
                        build_project_root.as_path(),
                        build_config_root.as_path(),
                        &build_projects,
                    )
                })
                .await;

            match build {
                Ok(hits) => {
                    service.coordinator().update_progress(&lease, 0.5);
                    if let Err(error) = write_knowledge_section_epoch(&service, &lease, &hits).await
                    {
                        service.coordinator().fail_build(
                            &lease,
                            format!("knowledge section epoch write failed: {error}"),
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

fn build_knowledge_section_rows(
    project_root: &Path,
    config_root: &Path,
    projects: &[UiProjectConfig],
) -> Vec<KnowledgeSectionRow> {
    let mut rows = Vec::new();
    let mut seen_files = HashSet::<PathBuf>::new();

    for root in configured_project_scan_roots(config_root, projects) {
        for entry in WalkDir::new(&root)
            .into_iter()
            .filter_entry(|entry| !should_skip_entry(entry))
        {
            let Ok(entry) = entry else { continue };
            if !entry.file_type().is_file() || !is_supported_note(entry.path()) {
                continue;
            }

            let canonical = entry
                .path()
                .canonicalize()
                .unwrap_or_else(|_| entry.path().to_path_buf());
            if !seen_files.insert(canonical) {
                continue;
            }

            let Ok(content) = std::fs::read_to_string(entry.path()) else {
                continue;
            };
            let Some(parsed) = parse_note(entry.path(), project_root, &content) else {
                continue;
            };
            let metadata = project_metadata_for_path(
                project_root,
                config_root,
                projects,
                parsed.doc.path.as_str(),
            );
            rows.extend(knowledge_rows_for_note(
                project_root,
                config_root,
                projects,
                &parsed,
                &metadata,
            ));
        }
    }

    rows
}

async fn write_knowledge_section_epoch(
    service: &SearchPlaneService,
    lease: &SearchBuildLease,
    rows: &[KnowledgeSectionRow],
) -> Result<(), VectorStoreError> {
    let store = service
        .open_store(SearchCorpusKind::KnowledgeSection)
        .await?;
    let table_name =
        SearchPlaneService::table_name(SearchCorpusKind::KnowledgeSection, lease.epoch);
    store
        .replace_record_batches(
            table_name.as_str(),
            knowledge_section_schema(),
            knowledge_section_batches(rows)?,
        )
        .await?;
    store
        .create_inverted_index(table_name.as_str(), search_text_column(), None)
        .await?;
    Ok(())
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
    let rows = build_knowledge_section_rows(project_root, config_root, projects);
    match write_knowledge_section_epoch(service, &lease, &rows).await {
        Ok(()) => {
            service.publish_ready_and_maintain(
                &lease,
                rows.len() as u64,
                fragment_count_from_len(rows.len()),
            );
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

fn should_skip_entry(entry: &DirEntry) -> bool {
    if !entry.file_type().is_dir() {
        return false;
    }

    matches!(
        entry.file_name().to_string_lossy().as_ref(),
        ".git"
            | ".cache"
            | ".devenv"
            | ".direnv"
            | ".run"
            | "target"
            | "node_modules"
            | "dist"
            | "coverage"
            | "__pycache__"
    )
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
