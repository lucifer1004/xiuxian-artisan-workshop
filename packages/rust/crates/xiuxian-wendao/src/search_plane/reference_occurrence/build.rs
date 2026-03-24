use std::collections::HashSet;
use std::path::Path;
use std::sync::LazyLock;

use regex::Regex;
use tokio::runtime::Handle;
use walkdir::{DirEntry, WalkDir};
use xiuxian_ast::Lang;
use xiuxian_vector::{ScalarIndexType, VectorStoreError};

use crate::gateway::studio::build_ast_index;
use crate::gateway::studio::search::project_scope::{
    configured_project_scan_roots, index_path_for_entry, project_metadata_for_path,
};
use crate::gateway::studio::search::support::infer_crate_name;
use crate::gateway::studio::types::{ReferenceSearchHit, StudioNavigationTarget, UiProjectConfig};
use crate::search_plane::{
    BeginBuildDecision, SearchBuildLease, SearchCorpusKind, SearchPlaneService,
};

use super::schema::{filter_column, reference_occurrence_batches, reference_occurrence_schema};

static REFERENCE_TOKEN_PATTERN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"[A-Za-z_][A-Za-z0-9_]*").unwrap_or_else(|error| {
        panic!("reference token regex must compile: {error}");
    })
});

#[cfg(test)]
#[derive(Debug, thiserror::Error)]
pub(crate) enum ReferenceOccurrenceBuildError {
    #[error(transparent)]
    Storage(#[from] VectorStoreError),
}

pub(crate) fn ensure_reference_occurrence_index_started(
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
        SearchCorpusKind::ReferenceOccurrence,
        fingerprint,
        SearchCorpusKind::ReferenceOccurrence.schema_version(),
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
            let build: Result<Vec<ReferenceSearchHit>, tokio::task::JoinError> =
                tokio::task::spawn_blocking(move || {
                    build_reference_occurrences(
                        build_project_root.as_path(),
                        build_config_root.as_path(),
                        &build_projects,
                    )
                })
                .await;

            match build {
                Ok(hits) => {
                    service.coordinator().update_progress(&lease, 0.55);
                    if let Err(error) =
                        write_reference_occurrence_epoch(&service, &lease, &hits).await
                    {
                        service.coordinator().fail_build(
                            &lease,
                            format!("reference occurrence epoch write failed: {error}"),
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
                        format!("reference occurrence background build panicked: {error}"),
                    );
                }
            }
        });
    } else {
        service.coordinator().fail_build(
            &lease,
            "Tokio runtime unavailable for reference occurrence index build",
        );
    }
}

fn build_reference_occurrences(
    project_root: &Path,
    config_root: &Path,
    projects: &[UiProjectConfig],
) -> Vec<ReferenceSearchHit> {
    let ast_hits = build_ast_index(project_root, config_root, projects);
    let definition_locations = ast_hits
        .iter()
        .map(|hit| {
            (
                hit.name.to_ascii_lowercase(),
                hit.path.clone(),
                hit.line_start,
            )
        })
        .collect::<HashSet<_>>();
    let mut hits = Vec::new();
    for root in configured_project_scan_roots(config_root, projects) {
        for entry in WalkDir::new(&root)
            .into_iter()
            .filter_entry(|entry| !should_skip_entry(entry))
        {
            let Ok(entry) = entry else { continue };
            if !entry.file_type().is_file() {
                continue;
            }

            let normalized_path = index_path_for_entry(project_root, entry.path());
            let normalized_path_ref = Path::new(normalized_path.as_str());
            let Some(language) = reference_scan_lang(normalized_path_ref) else {
                continue;
            };
            let crate_name = infer_crate_name(normalized_path_ref);
            let metadata = project_metadata_for_path(
                project_root,
                config_root,
                projects,
                normalized_path.as_str(),
            );

            if let Ok(content) = std::fs::read_to_string(entry.path()) {
                for (line_idx, line_text) in content.lines().enumerate() {
                    let line_number = line_idx + 1;
                    let mut seen_tokens = HashSet::new();
                    for mat in REFERENCE_TOKEN_PATTERN.find_iter(line_text) {
                        let token = mat.as_str();
                        let token_folded = token.to_ascii_lowercase();
                        if !seen_tokens.insert(token_folded.clone()) {
                            continue;
                        }
                        if definition_locations.contains(&(
                            token_folded,
                            normalized_path.clone(),
                            line_number,
                        )) {
                            continue;
                        }

                        let column = line_text[..mat.start()].chars().count() + 1;
                        hits.push(ReferenceSearchHit {
                            name: token.to_string(),
                            path: normalized_path.clone(),
                            language: language.to_string(),
                            crate_name: crate_name.clone(),
                            project_name: metadata.project_name.clone(),
                            root_label: metadata.root_label.clone(),
                            navigation_target: reference_navigation_target(
                                normalized_path.as_str(),
                                crate_name.as_str(),
                                metadata.project_name.as_deref(),
                                metadata.root_label.as_deref(),
                                line_number,
                                column,
                            ),
                            line: line_number,
                            column,
                            line_text: line_text.trim().to_string(),
                            score: 0.0,
                        });
                    }
                }
            }
        }
    }

    hits
}

async fn write_reference_occurrence_epoch(
    service: &SearchPlaneService,
    lease: &SearchBuildLease,
    hits: &[ReferenceSearchHit],
) -> Result<(), VectorStoreError> {
    let store = service
        .open_store(SearchCorpusKind::ReferenceOccurrence)
        .await?;
    let table_name =
        SearchPlaneService::table_name(SearchCorpusKind::ReferenceOccurrence, lease.epoch);
    let schema = reference_occurrence_schema();
    let batches = reference_occurrence_batches(hits)?;
    store
        .replace_record_batches(table_name.as_str(), schema, batches)
        .await?;
    store
        .create_column_scalar_index(
            table_name.as_str(),
            filter_column(),
            None,
            ScalarIndexType::BTree,
        )
        .await?;
    Ok(())
}

#[cfg(test)]
pub(crate) async fn publish_reference_occurrences_from_projects(
    service: &SearchPlaneService,
    project_root: &Path,
    config_root: &Path,
    projects: &[UiProjectConfig],
    fingerprint: &str,
) -> Result<(), ReferenceOccurrenceBuildError> {
    let lease = match service.coordinator().begin_build(
        SearchCorpusKind::ReferenceOccurrence,
        fingerprint,
        SearchCorpusKind::ReferenceOccurrence.schema_version(),
    ) {
        BeginBuildDecision::Started(lease) => lease,
        BeginBuildDecision::AlreadyReady(_) | BeginBuildDecision::AlreadyIndexing(_) => {
            return Ok(());
        }
    };
    let hits = build_reference_occurrences(project_root, config_root, projects);
    match write_reference_occurrence_epoch(service, &lease, &hits).await {
        Ok(()) => {
            service.publish_ready_and_maintain(
                &lease,
                hits.len() as u64,
                fragment_count_from_len(hits.len()),
            );
            Ok(())
        }
        Err(error) => {
            service.coordinator().fail_build(
                &lease,
                format!("reference occurrence epoch write failed: {error}"),
            );
            Err(ReferenceOccurrenceBuildError::Storage(error))
        }
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

fn reference_scan_lang(path: &Path) -> Option<&'static str> {
    match Lang::from_path(path)? {
        Lang::Python => Some("python"),
        Lang::Rust => Some("rust"),
        Lang::JavaScript => Some("javascript"),
        Lang::TypeScript => Some("typescript"),
        Lang::Bash => Some("bash"),
        Lang::Go => Some("go"),
        Lang::Java => Some("java"),
        Lang::C => Some("c"),
        Lang::Cpp => Some("cpp"),
        Lang::CSharp => Some("csharp"),
        Lang::Ruby => Some("ruby"),
        Lang::Swift => Some("swift"),
        Lang::Kotlin => Some("kotlin"),
        Lang::Lua => Some("lua"),
        Lang::Php => Some("php"),
        _ => None,
    }
}

fn reference_navigation_target(
    path: &str,
    crate_name: &str,
    project_name: Option<&str>,
    root_label: Option<&str>,
    line: usize,
    column: usize,
) -> StudioNavigationTarget {
    StudioNavigationTarget {
        path: path.to_string(),
        category: "doc".to_string(),
        project_name: project_name
            .map(ToString::to_string)
            .or_else(|| Some(crate_name.to_string())),
        root_label: root_label.map(ToString::to_string),
        line: Some(line),
        line_end: Some(line),
        column: Some(column),
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
