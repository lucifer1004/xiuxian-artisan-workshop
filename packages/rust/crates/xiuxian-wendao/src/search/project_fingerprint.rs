use std::collections::BTreeMap;
use std::path::Path;
use std::path::PathBuf;
use std::time::UNIX_EPOCH;

use walkdir::WalkDir;

use crate::gateway::studio::search::project_scope::{
    configured_project_scopes, index_path_for_entry,
};
use crate::gateway::studio::search::source_index::{
    ast_search_lang, is_markdown_path, should_skip_entry,
};
use crate::gateway::studio::types::UiProjectConfig;
use crate::parsers::markdown::is_supported_note;
use crate::search::SearchFileFingerprint;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ProjectFingerprintMode {
    Symbol,
    Source,
    Note,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ProjectScannedFile {
    pub(crate) scan_root: PathBuf,
    pub(crate) partition_id: String,
    pub(crate) absolute_path: PathBuf,
    pub(crate) normalized_path: String,
    pub(crate) project_name: Option<String>,
    pub(crate) root_label: Option<String>,
    pub(crate) size_bytes: u64,
    pub(crate) modified_secs: u64,
    pub(crate) modified_nanos: u32,
}

impl ProjectScannedFile {
    #[must_use]
    pub(crate) fn to_file_fingerprint(
        &self,
        extractor_version: u32,
        schema_version: u32,
    ) -> SearchFileFingerprint {
        SearchFileFingerprint {
            relative_path: self.normalized_path.clone(),
            partition_id: Some(self.partition_id.clone()),
            size_bytes: self.size_bytes,
            modified_unix_ms: self
                .modified_secs
                .saturating_mul(1_000)
                .saturating_add(u64::from(self.modified_nanos / 1_000_000)),
            extractor_version,
            schema_version,
            blake3: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct ProjectFileMetadata {
    path: String,
    size_bytes: u64,
    modified_secs: u64,
    modified_nanos: u32,
}

pub(crate) fn fingerprint_symbol_projects(
    project_root: &Path,
    config_root: &Path,
    projects: &[UiProjectConfig],
) -> String {
    let files = scan_symbol_project_files(project_root, config_root, projects);
    fingerprint_projects(
        project_root,
        config_root,
        projects,
        ProjectFingerprintMode::Symbol,
        &files,
    )
}

pub(crate) fn fingerprint_source_projects(
    project_root: &Path,
    config_root: &Path,
    projects: &[UiProjectConfig],
) -> String {
    let files = scan_source_project_files(project_root, config_root, projects);
    fingerprint_projects(
        project_root,
        config_root,
        projects,
        ProjectFingerprintMode::Source,
        &files,
    )
}

pub(crate) fn fingerprint_note_projects(
    project_root: &Path,
    config_root: &Path,
    projects: &[UiProjectConfig],
) -> String {
    let files = scan_note_project_files(project_root, config_root, projects);
    fingerprint_projects(
        project_root,
        config_root,
        projects,
        ProjectFingerprintMode::Note,
        &files,
    )
}

pub(crate) fn scan_symbol_project_files(
    project_root: &Path,
    config_root: &Path,
    projects: &[UiProjectConfig],
) -> Vec<ProjectScannedFile> {
    project_files(
        project_root,
        config_root,
        projects,
        ProjectFingerprintMode::Symbol,
    )
}

pub(crate) fn scan_source_project_files(
    project_root: &Path,
    config_root: &Path,
    projects: &[UiProjectConfig],
) -> Vec<ProjectScannedFile> {
    project_files(
        project_root,
        config_root,
        projects,
        ProjectFingerprintMode::Source,
    )
}

pub(crate) fn scan_note_project_files(
    project_root: &Path,
    config_root: &Path,
    projects: &[UiProjectConfig],
) -> Vec<ProjectScannedFile> {
    project_files(
        project_root,
        config_root,
        projects,
        ProjectFingerprintMode::Note,
    )
}

fn fingerprint_projects(
    project_root: &Path,
    config_root: &Path,
    projects: &[UiProjectConfig],
    mode: ProjectFingerprintMode,
    files: &[ProjectScannedFile],
) -> String {
    let mut hasher = blake3::Hasher::new();
    hasher.update(project_root.to_string_lossy().as_bytes());
    hasher.update(config_root.to_string_lossy().as_bytes());
    hasher.update(match mode {
        ProjectFingerprintMode::Symbol => b"symbol",
        ProjectFingerprintMode::Source => b"source",
        ProjectFingerprintMode::Note => b"note",
    });
    for project in projects {
        hasher.update(project.name.as_bytes());
        hasher.update(project.root.as_bytes());
        for dir in &project.dirs {
            hasher.update(dir.as_bytes());
        }
    }
    for file in project_file_metadata(files) {
        hasher.update(file.path.as_bytes());
        hasher.update(&file.size_bytes.to_le_bytes());
        hasher.update(&file.modified_secs.to_le_bytes());
        hasher.update(&file.modified_nanos.to_le_bytes());
    }
    hasher.finalize().to_hex().to_string()
}

fn project_file_metadata(files: &[ProjectScannedFile]) -> Vec<ProjectFileMetadata> {
    files
        .iter()
        .map(|file| ProjectFileMetadata {
            path: file.normalized_path.clone(),
            size_bytes: file.size_bytes,
            modified_secs: file.modified_secs,
            modified_nanos: file.modified_nanos,
        })
        .collect()
}

fn project_files(
    project_root: &Path,
    config_root: &Path,
    projects: &[UiProjectConfig],
    mode: ProjectFingerprintMode,
) -> Vec<ProjectScannedFile> {
    let mut files = BTreeMap::<String, ProjectScannedFile>::new();
    for scope in configured_project_scopes(config_root, projects) {
        for entry in WalkDir::new(scope.scope_path.as_path())
            .into_iter()
            .filter_entry(|entry| !should_skip_entry(entry))
        {
            let Ok(entry) = entry else { continue };
            if !entry.file_type().is_file() {
                continue;
            }

            let normalized_path = index_path_for_entry(project_root, entry.path());
            if !matches_mode(mode, Path::new(normalized_path.as_str())) {
                continue;
            }

            let Ok(metadata) = entry.metadata() else {
                continue;
            };
            let (modified_secs, modified_nanos) = metadata
                .modified()
                .ok()
                .and_then(|value| value.duration_since(UNIX_EPOCH).ok())
                .map_or((0, 0), |duration| {
                    (duration.as_secs(), duration.subsec_nanos())
                });

            files
                .entry(normalized_path.clone())
                .or_insert(ProjectScannedFile {
                    scan_root: scope.scope_path.clone(),
                    partition_id: scope.partition_id(),
                    absolute_path: entry.path().to_path_buf(),
                    normalized_path,
                    project_name: Some(scope.project_name.clone()),
                    root_label: scope.root_label.clone(),
                    size_bytes: metadata.len(),
                    modified_secs,
                    modified_nanos,
                });
        }
    }
    files.into_values().collect()
}

fn matches_mode(mode: ProjectFingerprintMode, path: &Path) -> bool {
    match mode {
        ProjectFingerprintMode::Symbol => is_markdown_path(path) || ast_search_lang(path).is_some(),
        ProjectFingerprintMode::Source => ast_search_lang(path).is_some(),
        ProjectFingerprintMode::Note => is_supported_note(path),
    }
}

#[cfg(test)]
#[path = "../../tests/unit/search/project_fingerprint.rs"]
mod tests;
