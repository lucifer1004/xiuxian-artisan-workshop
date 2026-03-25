use std::collections::{BTreeMap, BTreeSet, HashSet};
use std::path::Path;
use std::sync::LazyLock;

use regex::Regex;
use tokio::runtime::Handle;
use xiuxian_ast::Lang;
use xiuxian_vector::{ScalarIndexType, VectorStoreError};

use crate::gateway::studio::search::project_scope::project_metadata_for_path;
use crate::gateway::studio::search::source_index::build_ast_hits_for_file;
use crate::gateway::studio::search::support::infer_crate_name;
use crate::gateway::studio::types::{ReferenceSearchHit, StudioNavigationTarget, UiProjectConfig};
use crate::search_plane::{
    BeginBuildDecision, ProjectScannedFile, SearchBuildLease, SearchCorpusKind,
    SearchFileFingerprint, SearchPlaneService, delete_paths_from_table,
    fingerprint_source_projects, scan_source_project_files,
};

use super::schema::{
    filter_column, path_column, projected_columns, reference_occurrence_batches,
    reference_occurrence_schema,
};

static REFERENCE_TOKEN_PATTERN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"[A-Za-z_][A-Za-z0-9_]*").unwrap_or_else(|error| {
        panic!("reference token regex must compile: {error}");
    })
});

const REFERENCE_OCCURRENCE_EXTRACTOR_VERSION: u32 = 1;

#[derive(Debug, Clone)]
struct ReferenceOccurrenceBuildPlan {
    base_epoch: Option<u64>,
    file_fingerprints: BTreeMap<String, SearchFileFingerprint>,
    replaced_paths: BTreeSet<String>,
    changed_hits: Vec<ReferenceSearchHit>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ReferenceOccurrenceWriteResult {
    row_count: u64,
    fragment_count: u64,
}

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
    let active_epoch = service.corpus_active_epoch(SearchCorpusKind::ReferenceOccurrence);
    let service = service.clone();

    if let Ok(handle) = Handle::try_current() {
        handle.spawn(async move {
            let previous_fingerprints = service
                .corpus_file_fingerprints(SearchCorpusKind::ReferenceOccurrence)
                .await;
            let build: Result<ReferenceOccurrenceBuildPlan, tokio::task::JoinError> =
                tokio::task::spawn_blocking(move || {
                    plan_reference_occurrence_build(
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
                    let write = write_reference_occurrence_epoch(&service, &lease, &plan).await;
                    if let Err(error) = write {
                        service.coordinator().fail_build(
                            &lease,
                            format!("reference occurrence epoch write failed: {error}"),
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
                            format!("reference occurrence epoch prewarm failed: {error}"),
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
                                SearchCorpusKind::ReferenceOccurrence,
                                &plan.file_fingerprints,
                            )
                            .await;
                    }
                    service.coordinator().update_progress(&lease, 1.0);
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

fn plan_reference_occurrence_build(
    project_root: &Path,
    config_root: &Path,
    projects: &[UiProjectConfig],
    active_epoch: Option<u64>,
    previous_fingerprints: BTreeMap<String, SearchFileFingerprint>,
) -> ReferenceOccurrenceBuildPlan {
    let scanned_files = scan_source_project_files(project_root, config_root, projects);
    let file_fingerprints = scanned_files
        .iter()
        .map(|file| {
            (
                file.normalized_path.clone(),
                file.to_file_fingerprint(
                    REFERENCE_OCCURRENCE_EXTRACTOR_VERSION,
                    SearchCorpusKind::ReferenceOccurrence.schema_version(),
                ),
            )
        })
        .collect::<BTreeMap<_, _>>();

    let can_incremental_reuse = active_epoch.is_some() && !previous_fingerprints.is_empty();
    if !can_incremental_reuse {
        return ReferenceOccurrenceBuildPlan {
            base_epoch: None,
            file_fingerprints,
            replaced_paths: BTreeSet::new(),
            changed_hits: build_reference_occurrences_for_files(
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

    ReferenceOccurrenceBuildPlan {
        base_epoch: active_epoch,
        file_fingerprints,
        replaced_paths,
        changed_hits: build_reference_occurrences_for_files(
            project_root,
            config_root,
            projects,
            &changed_files,
        ),
    }
}

async fn write_reference_occurrence_epoch(
    service: &SearchPlaneService,
    lease: &SearchBuildLease,
    plan: &ReferenceOccurrenceBuildPlan,
) -> Result<ReferenceOccurrenceWriteResult, VectorStoreError> {
    let store = service
        .open_store(SearchCorpusKind::ReferenceOccurrence)
        .await?;
    let table_name =
        SearchPlaneService::table_name(SearchCorpusKind::ReferenceOccurrence, lease.epoch);
    let schema = reference_occurrence_schema();
    let changed_batches = reference_occurrence_batches(plan.changed_hits.as_slice())?;
    if let Some(base_epoch) = plan.base_epoch {
        let base_table_name =
            SearchPlaneService::table_name(SearchCorpusKind::ReferenceOccurrence, base_epoch);
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
        .create_column_scalar_index(
            table_name.as_str(),
            filter_column(),
            None,
            ScalarIndexType::BTree,
        )
        .await?;
    let table_info = store.get_table_info(table_name.as_str()).await?;
    Ok(ReferenceOccurrenceWriteResult {
        row_count: table_info.num_rows,
        fragment_count: u64::try_from(table_info.fragment_count).unwrap_or(u64::MAX),
    })
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
    let plan =
        plan_reference_occurrence_build(project_root, config_root, projects, None, BTreeMap::new());
    match write_reference_occurrence_epoch(service, &lease, &plan).await {
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
                format!("reference occurrence epoch write failed: {error}"),
            );
            Err(ReferenceOccurrenceBuildError::Storage(error))
        }
    }
}

fn build_reference_occurrences_for_files(
    project_root: &Path,
    config_root: &Path,
    projects: &[UiProjectConfig],
    files: &[ProjectScannedFile],
) -> Vec<ReferenceSearchHit> {
    let mut hits = Vec::new();
    for file in files {
        hits.extend(build_reference_occurrences_for_file(
            project_root,
            config_root,
            projects,
            file,
        ));
    }
    hits
}

fn build_reference_occurrences_for_file(
    project_root: &Path,
    config_root: &Path,
    projects: &[UiProjectConfig],
    file: &ProjectScannedFile,
) -> Vec<ReferenceSearchHit> {
    let normalized_path_ref = Path::new(file.normalized_path.as_str());
    let Some(language) = reference_scan_lang(normalized_path_ref) else {
        return Vec::new();
    };
    let metadata = project_metadata_for_path(
        project_root,
        config_root,
        projects,
        file.normalized_path.as_str(),
    );
    let crate_name = infer_crate_name(normalized_path_ref);
    let definition_locations = build_ast_hits_for_file(
        project_root,
        file.scan_root.as_path(),
        file.absolute_path.as_path(),
    )
    .into_iter()
    .map(|hit| (hit.name.to_ascii_lowercase(), hit.path, hit.line_start))
    .collect::<HashSet<_>>();

    let Ok(content) = std::fs::read_to_string(file.absolute_path.as_path()) else {
        return Vec::new();
    };
    let mut hits = Vec::new();
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
                file.normalized_path.clone(),
                line_number,
            )) {
                continue;
            }

            let column = line_text[..mat.start()].chars().count() + 1;
            hits.push(ReferenceSearchHit {
                name: token.to_string(),
                path: file.normalized_path.clone(),
                language: language.to_string(),
                crate_name: crate_name.clone(),
                project_name: metadata.project_name.clone(),
                root_label: metadata.root_label.clone(),
                navigation_target: reference_navigation_target(
                    file.normalized_path.as_str(),
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
    hits
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

fn fingerprint_projects(
    project_root: &Path,
    config_root: &Path,
    projects: &[UiProjectConfig],
) -> String {
    fingerprint_source_projects(project_root, config_root, projects)
}

#[cfg(test)]
mod tests {
    use std::collections::{BTreeMap, BTreeSet};
    use std::time::Duration;

    use super::{fingerprint_projects, plan_reference_occurrence_build};
    use crate::gateway::studio::types::UiProjectConfig;
    use crate::search_plane::cache::SearchPlaneCache;
    use crate::search_plane::reference_occurrence::search_reference_occurrences;
    use crate::search_plane::{
        SearchCorpusKind, SearchMaintenancePolicy, SearchManifestKeyspace, SearchPlanePhase,
        SearchPlaneService,
    };

    #[test]
    fn plan_reference_occurrence_build_only_reparses_changed_files() {
        let temp_dir = tempfile::tempdir().unwrap_or_else(|error| panic!("temp dir: {error}"));
        let project_root = temp_dir.path();
        std::fs::create_dir_all(project_root.join("src"))
            .unwrap_or_else(|error| panic!("create src: {error}"));
        std::fs::write(
            project_root.join("src/lib.rs"),
            "fn alpha() {}\nfn use_alpha() { alpha(); }\n",
        )
        .unwrap_or_else(|error| panic!("write lib: {error}"));
        std::fs::write(
            project_root.join("src/extra.rs"),
            "fn gamma() {}\nfn use_gamma() { gamma(); }\n",
        )
        .unwrap_or_else(|error| panic!("write extra: {error}"));
        let projects = vec![UiProjectConfig {
            name: "demo".to_string(),
            root: ".".to_string(),
            dirs: vec![".".to_string()],
        }];

        let first = plan_reference_occurrence_build(
            project_root,
            project_root,
            &projects,
            None,
            BTreeMap::new(),
        );
        assert_eq!(first.base_epoch, None);
        assert!(
            first
                .changed_hits
                .iter()
                .any(|hit| hit.path == "src/lib.rs" && hit.name == "alpha")
        );
        assert!(
            first
                .changed_hits
                .iter()
                .any(|hit| hit.path == "src/extra.rs" && hit.name == "gamma")
        );

        std::thread::sleep(Duration::from_millis(5));
        std::fs::write(
            project_root.join("src/lib.rs"),
            "fn beta() {}\nfn use_beta() { beta(); }\n",
        )
        .unwrap_or_else(|error| panic!("rewrite lib: {error}"));

        let second = plan_reference_occurrence_build(
            project_root,
            project_root,
            &projects,
            Some(7),
            first.file_fingerprints.clone(),
        );
        assert_eq!(second.base_epoch, Some(7));
        assert_eq!(
            second.replaced_paths,
            BTreeSet::from(["src/lib.rs".to_string()])
        );
        assert!(
            second
                .changed_hits
                .iter()
                .all(|hit| hit.path == "src/lib.rs")
        );
        assert!(
            second.changed_hits.iter().any(|hit| hit.name == "beta"),
            "changed-file rebuild must include the updated token"
        );
        assert!(
            second.changed_hits.iter().all(|hit| hit.name != "gamma"),
            "unchanged file rows must not be reparsed into the changed set"
        );
    }

    #[tokio::test]
    async fn reference_occurrence_incremental_refresh_reuses_unchanged_rows() {
        let temp_dir = tempfile::tempdir().unwrap_or_else(|error| panic!("temp dir: {error}"));
        let project_root = temp_dir.path().join("workspace");
        let storage_root = temp_dir.path().join("search_plane");
        std::fs::create_dir_all(project_root.join("src"))
            .unwrap_or_else(|error| panic!("create src: {error}"));
        std::fs::write(
            project_root.join("src/lib.rs"),
            "fn alpha() {}\nfn use_alpha() { alpha(); }\n",
        )
        .unwrap_or_else(|error| panic!("write lib: {error}"));
        std::fs::write(
            project_root.join("src/extra.rs"),
            "fn gamma() {}\nfn use_gamma() { gamma(); }\n",
        )
        .unwrap_or_else(|error| panic!("write extra: {error}"));
        let projects = vec![UiProjectConfig {
            name: "demo".to_string(),
            root: ".".to_string(),
            dirs: vec![".".to_string()],
        }];
        let keyspace = SearchManifestKeyspace::new(
            "xiuxian:test:search_plane:reference-occurrence-incremental",
        );
        let cache = SearchPlaneCache::for_tests(keyspace.clone());
        let service = SearchPlaneService::with_runtime(
            project_root.clone(),
            storage_root,
            keyspace,
            SearchMaintenancePolicy::default(),
            cache,
        );

        super::ensure_reference_occurrence_index_started(
            &service,
            project_root.as_path(),
            project_root.as_path(),
            &projects,
        );
        wait_for_reference_occurrence_ready(&service, None).await;

        let initial_gamma = search_reference_occurrences(&service, "gamma", 10)
            .await
            .unwrap_or_else(|error| panic!("query gamma: {error}"));
        assert_eq!(initial_gamma.len(), 1);
        let initial_alpha = search_reference_occurrences(&service, "alpha", 10)
            .await
            .unwrap_or_else(|error| panic!("query alpha: {error}"));
        assert_eq!(initial_alpha.len(), 1);

        std::fs::write(
            project_root.join("src/lib.rs"),
            "fn beta() {}\nfn use_beta() { beta(); }\n",
        )
        .unwrap_or_else(|error| panic!("rewrite lib: {error}"));
        super::ensure_reference_occurrence_index_started(
            &service,
            project_root.as_path(),
            project_root.as_path(),
            &projects,
        );
        wait_for_reference_occurrence_ready(&service, Some(1)).await;

        let gamma = search_reference_occurrences(&service, "gamma", 10)
            .await
            .unwrap_or_else(|error| panic!("query gamma after refresh: {error}"));
        assert_eq!(gamma.len(), 1);
        let beta = search_reference_occurrences(&service, "beta", 10)
            .await
            .unwrap_or_else(|error| panic!("query beta after refresh: {error}"));
        assert_eq!(beta.len(), 1);
        let alpha = search_reference_occurrences(&service, "alpha", 10)
            .await
            .unwrap_or_else(|error| panic!("query alpha after refresh: {error}"));
        assert!(alpha.is_empty());
    }

    async fn wait_for_reference_occurrence_ready(
        service: &SearchPlaneService,
        previous_epoch: Option<u64>,
    ) {
        for _ in 0..100 {
            let status = service
                .coordinator()
                .status_for(SearchCorpusKind::ReferenceOccurrence);
            if status.phase == SearchPlanePhase::Ready
                && status.active_epoch.is_some()
                && previous_epoch
                    .is_none_or(|epoch| status.active_epoch.unwrap_or_default() > epoch)
            {
                return;
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
        panic!("reference occurrence build did not reach ready state");
    }

    #[test]
    fn fingerprint_projects_changes_when_scanned_file_metadata_changes() {
        let temp_dir = tempfile::tempdir().unwrap_or_else(|error| panic!("temp dir: {error}"));
        let project_root = temp_dir.path();
        std::fs::create_dir_all(project_root.join("src"))
            .unwrap_or_else(|error| panic!("create src: {error}"));
        std::fs::create_dir_all(project_root.join("node_modules/pkg"))
            .unwrap_or_else(|error| panic!("create skipped dir: {error}"));
        std::fs::write(
            project_root.join("src/lib.rs"),
            "fn alpha() {}\nfn use_alpha() { alpha(); }\n",
        )
        .unwrap_or_else(|error| panic!("write rust source: {error}"));
        std::fs::write(
            project_root.join("node_modules/pkg/index.js"),
            "ignored();\n",
        )
        .unwrap_or_else(|error| panic!("write skipped file: {error}"));

        let projects = vec![UiProjectConfig {
            name: "demo".to_string(),
            root: ".".to_string(),
            dirs: vec![".".to_string()],
        }];

        let first = fingerprint_projects(project_root, project_root, &projects);
        std::fs::write(
            project_root.join("node_modules/pkg/index.js"),
            "ignored-again();\n",
        )
        .unwrap_or_else(|error| panic!("rewrite skipped file: {error}"));
        let after_skipped_change = fingerprint_projects(project_root, project_root, &projects);
        assert_eq!(first, after_skipped_change);

        std::fs::write(
            project_root.join("src/lib.rs"),
            "fn beta() {}\nfn use_beta() { beta(); }\n",
        )
        .unwrap_or_else(|error| panic!("rewrite rust source: {error}"));
        let second = fingerprint_projects(project_root, project_root, &projects);
        assert_ne!(first, second);
    }
}
