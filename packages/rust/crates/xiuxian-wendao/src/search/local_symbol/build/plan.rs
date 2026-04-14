use std::collections::BTreeMap;
use std::path::Path;

use crate::gateway::studio::types::UiProjectConfig;
use crate::search::local_symbol::build::LocalSymbolBuildPlan;
use crate::search::local_symbol::build::partitions::{
    build_hits_for_file, build_partition_plans_from_file_hits,
};
use crate::search::{
    ProjectScannedFile, SearchCorpusKind, SearchFileFingerprint, SearchPlaneService,
    ast_hits_fingerprint,
};
#[cfg(test)]
use crate::search::{fingerprint_symbol_projects, scan_symbol_project_files};

const LOCAL_SYMBOL_EXTRACTOR_VERSION: u32 = 1;

#[cfg(test)]
pub(crate) fn plan_local_symbol_build(
    service: &SearchPlaneService,
    project_root: &Path,
    config_root: &Path,
    projects: &[UiProjectConfig],
    active_epoch: Option<u64>,
    previous_fingerprints: &BTreeMap<String, SearchFileFingerprint>,
) -> LocalSymbolBuildPlan {
    let scanned_files = scan_symbol_project_files(project_root, config_root, projects);
    service.record_repeat_work_scanned_files(
        "local_symbol.plan",
        "scan_symbol_project_files",
        &scanned_files,
    );
    plan_local_symbol_build_with_scanned_files(
        service,
        project_root,
        config_root,
        projects,
        scanned_files.as_slice(),
        active_epoch,
        previous_fingerprints,
    )
}

pub(crate) fn plan_local_symbol_build_with_scanned_files(
    service: &SearchPlaneService,
    project_root: &Path,
    _config_root: &Path,
    _projects: &[UiProjectConfig],
    scanned_files: &[ProjectScannedFile],
    active_epoch: Option<u64>,
    previous_fingerprints: &BTreeMap<String, SearchFileFingerprint>,
) -> LocalSymbolBuildPlan {
    let can_incremental_reuse = active_epoch.is_some() && !previous_fingerprints.is_empty();
    let files_requiring_semantic_eval = if can_incremental_reuse {
        scanned_files
            .iter()
            .filter(|file| {
                previous_fingerprints
                    .get(file.normalized_path.as_str())
                    .is_none_or(|previous| {
                        !previous.matches_scan_metadata(
                            Some(file.partition_id.as_str()),
                            file.size_bytes,
                            file.modified_unix_ms(),
                            LOCAL_SYMBOL_EXTRACTOR_VERSION,
                            SearchCorpusKind::LocalSymbol.schema_version(),
                        )
                    })
            })
            .cloned()
            .collect::<Vec<_>>()
    } else {
        scanned_files.to_vec()
    };
    let markdown_snapshot = service
        .shared_markdown_project_snapshot(project_root, files_requiring_semantic_eval.as_slice());

    let mut file_fingerprints = BTreeMap::<String, SearchFileFingerprint>::new();
    let mut changed_files = Vec::<ProjectScannedFile>::new();
    let mut changed_file_hits =
        BTreeMap::<String, Vec<crate::gateway::studio::types::AstSearchHit>>::new();

    for file in scanned_files {
        if can_incremental_reuse
            && let Some(previous) = previous_fingerprints.get(file.normalized_path.as_str())
            && previous.matches_scan_metadata(
                Some(file.partition_id.as_str()),
                file.size_bytes,
                file.modified_unix_ms(),
                LOCAL_SYMBOL_EXTRACTOR_VERSION,
                SearchCorpusKind::LocalSymbol.schema_version(),
            )
        {
            file_fingerprints.insert(file.normalized_path.clone(), previous.clone());
            continue;
        }

        let file_hits = build_hits_for_file(service, project_root, file, &markdown_snapshot);
        let fingerprint = file.to_semantic_file_fingerprint(
            LOCAL_SYMBOL_EXTRACTOR_VERSION,
            SearchCorpusKind::LocalSymbol.schema_version(),
            ast_hits_fingerprint(&file_hits),
        );
        let changed = !can_incremental_reuse
            || previous_fingerprints
                .get(file.normalized_path.as_str())
                .is_none_or(|previous| !previous.equivalent_for_incremental(&fingerprint));
        file_fingerprints.insert(file.normalized_path.clone(), fingerprint);
        if changed {
            changed_files.push(file.clone());
            changed_file_hits.insert(file.normalized_path.clone(), file_hits);
        }
    }

    if !can_incremental_reuse {
        return LocalSymbolBuildPlan {
            base_epoch: None,
            file_fingerprints,
            partitions: build_partition_plans_from_file_hits(scanned_files, &changed_file_hits),
        };
    }

    let mut partitions =
        build_partition_plans_from_file_hits(changed_files.as_slice(), &changed_file_hits);
    for file in &changed_files {
        partitions
            .entry(file.partition_id.clone())
            .or_default()
            .replaced_paths
            .insert(file.normalized_path.clone());
    }
    for (path, previous_fingerprint) in previous_fingerprints {
        let current_fingerprint = file_fingerprints.get(path.as_str());
        if current_fingerprint.is_none() {
            if let Some(partition_id) = previous_fingerprint.partition_id.as_deref() {
                partitions
                    .entry(partition_id.to_string())
                    .or_default()
                    .replaced_paths
                    .insert(path.clone());
            }
            continue;
        }

        if let Some(current_fingerprint) = current_fingerprint
            && current_fingerprint.partition_id != previous_fingerprint.partition_id
            && let Some(partition_id) = previous_fingerprint.partition_id.as_deref()
        {
            partitions
                .entry(partition_id.to_string())
                .or_default()
                .replaced_paths
                .insert(path.clone());
        }
    }

    LocalSymbolBuildPlan {
        base_epoch: active_epoch,
        file_fingerprints,
        partitions,
    }
}

#[cfg(test)]
pub(crate) fn fingerprint_projects(
    project_root: &Path,
    config_root: &Path,
    projects: &[UiProjectConfig],
) -> String {
    fingerprint_symbol_projects(project_root, config_root, projects)
}
