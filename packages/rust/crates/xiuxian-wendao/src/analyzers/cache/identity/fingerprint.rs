use std::fs;
use std::path::{Path, PathBuf};

use walkdir::{DirEntry, WalkDir};

use super::classify::{FingerprintMode, analysis_fingerprint_mode};

pub(crate) fn collect_repository_analysis_identity(
    repository_root: &Path,
    plugin_ids: &[String],
) -> Option<String> {
    if !repository_root.is_dir() {
        return None;
    }

    let mut relevant_files = WalkDir::new(repository_root)
        .into_iter()
        .filter_entry(|entry| !should_skip_walk_entry(entry))
        .filter_map(Result::ok)
        .filter_map(|entry| relevant_file(repository_root, entry, plugin_ids))
        .collect::<Vec<_>>();

    relevant_files.sort_by(|left, right| left.relative_path.cmp(&right.relative_path));

    let mut hasher = blake3::Hasher::new();
    hasher.update(b"xiuxian_wendao.repo_analysis_identity.v1\0");

    if relevant_files.is_empty() {
        hasher.update(b"empty\0");
        return Some(hasher.finalize().to_hex().to_string());
    }

    for file in relevant_files {
        hasher.update(file.relative_path.as_bytes());
        hasher.update(b"\0");
        hasher.update(match file.mode {
            FingerprintMode::PathOnly => b"path",
            FingerprintMode::Contents => b"contents",
        });
        hasher.update(b"\0");
        if matches!(file.mode, FingerprintMode::Contents) {
            let contents = fs::read(&file.absolute_path).ok()?;
            hasher.update(contents.as_slice());
            hasher.update(b"\0");
        }
    }

    Some(hasher.finalize().to_hex().to_string())
}

#[derive(Debug)]
struct RelevantFile {
    relative_path: String,
    absolute_path: PathBuf,
    mode: FingerprintMode,
}

fn relevant_file(
    repository_root: &Path,
    entry: DirEntry,
    plugin_ids: &[String],
) -> Option<RelevantFile> {
    if !entry.file_type().is_file() {
        return None;
    }

    let relative_path = entry
        .path()
        .strip_prefix(repository_root)
        .ok()?
        .to_string_lossy()
        .replace('\\', "/");
    let mode = analysis_fingerprint_mode(relative_path.as_str(), plugin_ids)?;

    Some(RelevantFile {
        relative_path,
        absolute_path: entry.into_path(),
        mode,
    })
}

fn should_skip_walk_entry(entry: &DirEntry) -> bool {
    entry.depth() > 0
        && entry
            .file_name()
            .to_str()
            .is_some_and(|name| name == ".git")
}
