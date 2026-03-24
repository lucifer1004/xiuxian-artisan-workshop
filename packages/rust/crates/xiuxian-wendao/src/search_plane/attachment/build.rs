use std::collections::HashSet;
use std::path::{Path, PathBuf};

use tokio::runtime::Handle;
use walkdir::{DirEntry, WalkDir};
use xiuxian_vector::{ScalarIndexType, VectorStoreError};

use crate::gateway::studio::search::project_scope::{
    configured_project_scan_roots, project_metadata_for_path,
};
use crate::gateway::studio::types::{AttachmentSearchHit, StudioNavigationTarget, UiProjectConfig};
use crate::link_graph::LinkGraphAttachmentKind;
use crate::link_graph::parser::{is_supported_note, parse_note};
use crate::search_plane::{
    BeginBuildDecision, SearchBuildLease, SearchCorpusKind, SearchPlaneService,
};

use super::schema::{
    attachment_batches, attachment_ext_column, attachment_schema, kind_column, search_text_column,
};

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
    let service = service.clone();

    if let Ok(handle) = Handle::try_current() {
        handle.spawn(async move {
            let build: Result<Vec<AttachmentSearchHit>, tokio::task::JoinError> =
                tokio::task::spawn_blocking(move || {
                    build_attachment_hits(
                        build_project_root.as_path(),
                        build_config_root.as_path(),
                        &build_projects,
                    )
                })
                .await;

            match build {
                Ok(hits) => {
                    service.coordinator().update_progress(&lease, 0.5);
                    if let Err(error) = write_attachment_epoch(&service, &lease, &hits).await {
                        service
                            .coordinator()
                            .fail_build(&lease, format!("attachment epoch write failed: {error}"));
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

fn build_attachment_hits(
    project_root: &Path,
    config_root: &Path,
    projects: &[UiProjectConfig],
) -> Vec<AttachmentSearchHit> {
    let mut hits = Vec::new();
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
            hits.extend(attachment_hits_for_parsed_note(
                &parsed,
                metadata.project_name.as_deref(),
                metadata.root_label.as_deref(),
            ));
        }
    }

    hits
}

async fn write_attachment_epoch(
    service: &SearchPlaneService,
    lease: &SearchBuildLease,
    hits: &[AttachmentSearchHit],
) -> Result<(), VectorStoreError> {
    let store = service.open_store(SearchCorpusKind::Attachment).await?;
    let table_name = SearchPlaneService::table_name(SearchCorpusKind::Attachment, lease.epoch);
    store
        .replace_record_batches(
            table_name.as_str(),
            attachment_schema(),
            attachment_batches(hits)?,
        )
        .await?;
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
    Ok(())
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
    let hits = build_attachment_hits(project_root, config_root, projects);
    match write_attachment_epoch(service, &lease, &hits).await {
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
                .fail_build(&lease, format!("attachment epoch write failed: {error}"));
            Err(AttachmentBuildError::Storage(error))
        }
    }
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
