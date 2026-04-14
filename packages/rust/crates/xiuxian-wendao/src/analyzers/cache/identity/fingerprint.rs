use std::fs;
use std::path::{Path, PathBuf};

use walkdir::{DirEntry, WalkDir};
use xiuxian_wendao_julia::{
    julia_parser_summary_file_semantic_fingerprint_for_repository,
    modelica_parser_summary_file_semantic_fingerprint_for_repository,
};

use super::classify::{FingerprintMode, analysis_fingerprint_mode};
use crate::analyzers::config::RegisteredRepository;

pub(crate) fn collect_repository_analysis_identity(
    repository: &RegisteredRepository,
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
            RelevantFileIdentityMode::PathOnly => b"path",
            RelevantFileIdentityMode::Contents => b"contents",
            RelevantFileIdentityMode::JuliaParserSummary => b"semantic:julia_parser_summary",
            RelevantFileIdentityMode::ModelicaParserSummary => b"semantic:modelica_parser_summary",
        });
        hasher.update(b"\0");
        match file.mode {
            RelevantFileIdentityMode::PathOnly => {}
            RelevantFileIdentityMode::Contents => {
                let contents = fs::read(&file.absolute_path).ok()?;
                hasher.update(contents.as_slice());
                hasher.update(b"\0");
            }
            RelevantFileIdentityMode::JuliaParserSummary => {
                hash_julia_parser_summary_identity(
                    repository,
                    &file.absolute_path,
                    file.relative_path.as_str(),
                    &mut hasher,
                )?;
            }
            RelevantFileIdentityMode::ModelicaParserSummary => {
                hash_modelica_parser_summary_identity(
                    repository,
                    &file.absolute_path,
                    file.relative_path.as_str(),
                    &mut hasher,
                )?;
            }
        }
    }

    Some(hasher.finalize().to_hex().to_string())
}

#[derive(Debug)]
struct RelevantFile {
    relative_path: String,
    absolute_path: PathBuf,
    mode: RelevantFileIdentityMode,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum RelevantFileIdentityMode {
    PathOnly,
    Contents,
    JuliaParserSummary,
    ModelicaParserSummary,
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
    let mode = relevant_file_identity_mode(relative_path.as_str(), plugin_ids)?;

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

fn relevant_file_identity_mode(
    relative_path: &str,
    plugin_ids: &[String],
) -> Option<RelevantFileIdentityMode> {
    let mode = analysis_fingerprint_mode(relative_path, plugin_ids)?;
    if matches!(mode, FingerprintMode::Contents)
        && should_use_julia_parser_summary_identity(relative_path, plugin_ids)
    {
        return Some(RelevantFileIdentityMode::JuliaParserSummary);
    }
    if matches!(mode, FingerprintMode::Contents)
        && should_use_modelica_parser_summary_identity(relative_path, plugin_ids)
    {
        return Some(RelevantFileIdentityMode::ModelicaParserSummary);
    }

    Some(match mode {
        FingerprintMode::PathOnly => RelevantFileIdentityMode::PathOnly,
        FingerprintMode::Contents => RelevantFileIdentityMode::Contents,
    })
}

fn should_use_julia_parser_summary_identity(relative_path: &str, plugin_ids: &[String]) -> bool {
    supports_semantic_parser_summary_identity(plugin_ids)
        && plugin_ids.iter().any(|plugin_id| plugin_id == "julia")
        && relative_path.starts_with("src/")
        && Path::new(relative_path)
            .extension()
            .and_then(std::ffi::OsStr::to_str)
            .is_some_and(|extension| extension == "jl")
}

fn should_use_modelica_parser_summary_identity(relative_path: &str, plugin_ids: &[String]) -> bool {
    supports_semantic_parser_summary_identity(plugin_ids)
        && plugin_ids.iter().any(|plugin_id| plugin_id == "modelica")
        && Path::new(relative_path)
            .extension()
            .and_then(std::ffi::OsStr::to_str)
            .is_some_and(|extension| extension.eq_ignore_ascii_case("mo"))
}

fn supports_semantic_parser_summary_identity(plugin_ids: &[String]) -> bool {
    !plugin_ids.is_empty()
        && plugin_ids
            .iter()
            .all(|plugin_id| matches!(plugin_id.as_str(), "julia" | "modelica"))
}

fn hash_julia_parser_summary_identity(
    repository: &RegisteredRepository,
    absolute_path: &Path,
    relative_path: &str,
    hasher: &mut blake3::Hasher,
) -> Option<()> {
    let contents = fs::read(absolute_path).ok()?;
    let semantic_fingerprint =
        std::str::from_utf8(contents.as_slice())
            .ok()
            .and_then(|source_text| {
                julia_parser_summary_file_semantic_fingerprint_for_repository(
                    repository,
                    relative_path,
                    source_text,
                )
                .ok()
            });
    match semantic_fingerprint {
        Some(fingerprint) => hasher.update(fingerprint.as_bytes()),
        None => hasher.update(contents.as_slice()),
    };
    hasher.update(b"\0");
    Some(())
}

fn hash_modelica_parser_summary_identity(
    repository: &RegisteredRepository,
    absolute_path: &Path,
    relative_path: &str,
    hasher: &mut blake3::Hasher,
) -> Option<()> {
    let contents = fs::read(absolute_path).ok()?;
    let semantic_fingerprint =
        std::str::from_utf8(contents.as_slice())
            .ok()
            .and_then(|source_text| {
                modelica_parser_summary_file_semantic_fingerprint_for_repository(
                    repository,
                    relative_path,
                    source_text,
                )
                .ok()
            });
    match semantic_fingerprint {
        Some(fingerprint) => hasher.update(fingerprint.as_bytes()),
        None => hasher.update(contents.as_slice()),
    };
    hasher.update(b"\0");
    Some(())
}
