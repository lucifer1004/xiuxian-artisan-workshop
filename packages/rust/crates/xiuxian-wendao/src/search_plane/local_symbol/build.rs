use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use tokio::runtime::Handle;
use xiuxian_vector::VectorStoreError;

use crate::gateway::studio::search::source_index::build_ast_hits_for_file;
use crate::gateway::studio::types::AstSearchHit;
use crate::gateway::studio::types::UiProjectConfig;
use crate::search_plane::{
    BeginBuildDecision, ProjectScannedFile, SearchBuildLease, SearchCorpusKind,
    SearchFileFingerprint, SearchPlaneService, delete_paths_from_table,
    fingerprint_symbol_projects, scan_symbol_project_files,
};

use super::schema::{local_symbol_batches, local_symbol_schema, path_column, projected_columns};

const LOCAL_SYMBOL_EXTRACTOR_VERSION: u32 = 1;

#[derive(Debug, Clone, Default)]
struct LocalSymbolPartitionBuildPlan {
    replaced_paths: BTreeSet<String>,
    changed_hits: Vec<AstSearchHit>,
}

#[derive(Debug, Clone)]
struct LocalSymbolBuildPlan {
    base_epoch: Option<u64>,
    file_fingerprints: BTreeMap<String, SearchFileFingerprint>,
    partitions: BTreeMap<String, LocalSymbolPartitionBuildPlan>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct LocalSymbolWriteResult {
    row_count: u64,
    fragment_count: u64,
}

#[cfg(test)]
#[derive(Debug, thiserror::Error)]
pub(crate) enum LocalSymbolBuildError {
    #[error("local symbol build was not started for fingerprint `{0}`")]
    BuildRejected(String),
    #[error(transparent)]
    Storage(#[from] VectorStoreError),
}

pub(crate) fn ensure_local_symbol_index_started(
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
        SearchCorpusKind::LocalSymbol,
        fingerprint,
        SearchCorpusKind::LocalSymbol.schema_version(),
    );
    let BeginBuildDecision::Started(lease) = decision else {
        return;
    };

    let build_projects = projects.to_vec();
    let build_project_root = project_root.to_path_buf();
    let build_config_root = config_root.to_path_buf();
    let active_epoch = service
        .corpus_active_epoch(SearchCorpusKind::LocalSymbol)
        .filter(|epoch| {
            service.local_epoch_has_partition_tables(SearchCorpusKind::LocalSymbol, *epoch)
        });
    let service = service.clone();

    if let Ok(handle) = Handle::try_current() {
        handle.spawn(async move {
            let previous_fingerprints = service
                .corpus_file_fingerprints(SearchCorpusKind::LocalSymbol)
                .await;
            let build: Result<LocalSymbolBuildPlan, tokio::task::JoinError> =
                tokio::task::spawn_blocking(move || {
                    plan_local_symbol_build(
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
                    let write = write_local_symbol_epoch(&service, &lease, &plan).await;
                    if let Err(error) = write {
                        service.coordinator().fail_build(
                            &lease,
                            format!("local symbol epoch write failed: {error}"),
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
                            format!("local symbol epoch prewarm failed: {error}"),
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
                                SearchCorpusKind::LocalSymbol,
                                &plan.file_fingerprints,
                            )
                            .await;
                    }
                    service.coordinator().update_progress(&lease, 1.0);
                }
                Err(error) => {
                    service.coordinator().fail_build(
                        &lease,
                        format!("local symbol background build panicked: {error}"),
                    );
                }
            }
        });
    } else {
        service.coordinator().fail_build(
            &lease,
            "Tokio runtime unavailable for local symbol index build",
        );
    }
}

async fn write_local_symbol_epoch(
    service: &SearchPlaneService,
    lease: &SearchBuildLease,
    plan: &LocalSymbolBuildPlan,
) -> Result<LocalSymbolWriteResult, VectorStoreError> {
    let store = service.open_store(SearchCorpusKind::LocalSymbol).await?;
    let schema = local_symbol_schema();
    let mut row_count = 0_u64;
    let mut fragment_count = 0_u64;

    for (partition_id, partition_plan) in &plan.partitions {
        let table_name = SearchPlaneService::local_partition_table_name(
            SearchCorpusKind::LocalSymbol,
            lease.epoch,
            partition_id.as_str(),
        );
        let changed_batches = local_symbol_batches(partition_plan.changed_hits.as_slice())?;

        if let Some(base_epoch) = plan.base_epoch {
            let base_table_name = SearchPlaneService::local_partition_table_name(
                SearchCorpusKind::LocalSymbol,
                base_epoch,
                partition_id.as_str(),
            );
            if service.local_table_exists(SearchCorpusKind::LocalSymbol, base_table_name.as_str()) {
                store
                    .clone_table(base_table_name.as_str(), table_name.as_str(), true)
                    .await?;
                delete_paths_from_table(
                    &store,
                    table_name.as_str(),
                    path_column(),
                    &partition_plan.replaced_paths,
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
            } else if !changed_batches.is_empty() {
                store
                    .replace_record_batches(table_name.as_str(), schema.clone(), changed_batches)
                    .await?;
            } else {
                continue;
            }
        } else if !changed_batches.is_empty() {
            store
                .replace_record_batches(table_name.as_str(), schema.clone(), changed_batches)
                .await?;
        } else {
            continue;
        }

        let table_info = store.get_table_info(table_name.as_str()).await?;
        if table_info.num_rows > 0 {
            store
                .create_inverted_index(table_name.as_str(), "search_text", None)
                .await?;
        }
        row_count = row_count.saturating_add(table_info.num_rows);
        fragment_count = fragment_count
            .saturating_add(u64::try_from(table_info.fragment_count).unwrap_or(u64::MAX));
    }

    Ok(LocalSymbolWriteResult {
        row_count,
        fragment_count,
    })
}

#[cfg(test)]
pub(crate) async fn publish_local_symbol_hits(
    service: &SearchPlaneService,
    fingerprint: &str,
    hits: &[AstSearchHit],
) -> Result<(), LocalSymbolBuildError> {
    let lease = match service.coordinator().begin_build(
        SearchCorpusKind::LocalSymbol,
        fingerprint,
        SearchCorpusKind::LocalSymbol.schema_version(),
    ) {
        BeginBuildDecision::Started(lease) => lease,
        BeginBuildDecision::AlreadyReady(_) | BeginBuildDecision::AlreadyIndexing(_) => {
            return Err(LocalSymbolBuildError::BuildRejected(
                fingerprint.to_string(),
            ));
        }
    };

    let plan = LocalSymbolBuildPlan {
        base_epoch: None,
        file_fingerprints: BTreeMap::new(),
        partitions: BTreeMap::from([(
            "manual".to_string(),
            LocalSymbolPartitionBuildPlan {
                replaced_paths: BTreeSet::new(),
                changed_hits: hits.to_vec(),
            },
        )]),
    };

    match write_local_symbol_epoch(service, &lease, &plan).await {
        Ok(write) => {
            let prewarm_columns = projected_columns();
            service
                .prewarm_epoch_table(lease.corpus, lease.epoch, &prewarm_columns)
                .await?;
            service.publish_ready_and_maintain(&lease, write.row_count, write.fragment_count);
            Ok(())
        }
        Err(error) => {
            service
                .coordinator()
                .fail_build(&lease, format!("local symbol epoch write failed: {error}"));
            Err(LocalSymbolBuildError::Storage(error))
        }
    }
}

fn plan_local_symbol_build(
    project_root: &Path,
    config_root: &Path,
    projects: &[UiProjectConfig],
    active_epoch: Option<u64>,
    previous_fingerprints: BTreeMap<String, SearchFileFingerprint>,
) -> LocalSymbolBuildPlan {
    let scanned_files = scan_symbol_project_files(project_root, config_root, projects);
    let file_fingerprints = scanned_files
        .iter()
        .map(|file| {
            (
                file.normalized_path.clone(),
                file.to_file_fingerprint(
                    LOCAL_SYMBOL_EXTRACTOR_VERSION,
                    SearchCorpusKind::LocalSymbol.schema_version(),
                ),
            )
        })
        .collect::<BTreeMap<_, _>>();

    let can_incremental_reuse = active_epoch.is_some() && !previous_fingerprints.is_empty();
    if !can_incremental_reuse {
        return LocalSymbolBuildPlan {
            base_epoch: None,
            file_fingerprints,
            partitions: build_partition_plans(project_root, scanned_files.as_slice()),
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
    let mut partitions = build_partition_plans(project_root, changed_files.as_slice());
    for file in &changed_files {
        partitions
            .entry(file.partition_id.clone())
            .or_default()
            .replaced_paths
            .insert(file.normalized_path.clone());
    }
    for (path, previous_fingerprint) in &previous_fingerprints {
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

fn build_partition_plans(
    project_root: &Path,
    files: &[ProjectScannedFile],
) -> BTreeMap<String, LocalSymbolPartitionBuildPlan> {
    let mut files_by_partition = BTreeMap::<String, Vec<ProjectScannedFile>>::new();
    for file in files {
        files_by_partition
            .entry(file.partition_id.clone())
            .or_default()
            .push(file.clone());
    }

    files_by_partition
        .into_iter()
        .map(|(partition_id, partition_files)| {
            (
                partition_id,
                LocalSymbolPartitionBuildPlan {
                    replaced_paths: BTreeSet::new(),
                    changed_hits: build_hits_for_files(project_root, partition_files.as_slice()),
                },
            )
        })
        .collect()
}

fn build_hits_for_files(project_root: &Path, files: &[ProjectScannedFile]) -> Vec<AstSearchHit> {
    let mut hits = Vec::new();
    for file in files {
        let mut file_hits = build_ast_hits_for_file(
            project_root,
            file.scan_root.as_path(),
            file.absolute_path.as_path(),
        );
        for hit in &mut file_hits {
            if file.project_name.is_some() {
                hit.project_name.clone_from(&file.project_name);
                hit.navigation_target
                    .project_name
                    .clone_from(&file.project_name);
            }
            if file.root_label.is_some() {
                hit.root_label.clone_from(&file.root_label);
                hit.navigation_target
                    .root_label
                    .clone_from(&file.root_label);
            }
        }
        hits.extend(file_hits);
    }
    hits
}

fn fingerprint_projects(
    project_root: &Path,
    config_root: &Path,
    projects: &[UiProjectConfig],
) -> String {
    fingerprint_symbol_projects(project_root, config_root, projects)
}

#[cfg(test)]
mod tests {
    use std::collections::{BTreeMap, BTreeSet};
    use std::time::Duration;

    use super::{fingerprint_projects, plan_local_symbol_build};
    use crate::gateway::studio::types::UiProjectConfig;
    use crate::search_plane::SearchPlaneService;
    use crate::search_plane::cache::SearchPlaneCache;
    use crate::search_plane::local_symbol::search_local_symbols;
    use crate::search_plane::{SearchCorpusKind, SearchMaintenancePolicy, SearchManifestKeyspace};

    #[test]
    fn fingerprint_projects_changes_when_scanned_file_metadata_changes() {
        let temp_dir = tempfile::tempdir().unwrap_or_else(|error| panic!("temp dir: {error}"));
        let project_root = temp_dir.path();
        std::fs::create_dir_all(project_root.join("src"))
            .unwrap_or_else(|error| panic!("create src: {error}"));
        std::fs::create_dir_all(project_root.join("node_modules/pkg"))
            .unwrap_or_else(|error| panic!("create skipped dir: {error}"));
        std::fs::write(project_root.join("src/lib.rs"), "fn alpha() {}\n")
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
            "fn alpha() {}\nfn beta() {}\n",
        )
        .unwrap_or_else(|error| panic!("rewrite rust source: {error}"));
        let second = fingerprint_projects(project_root, project_root, &projects);
        assert_ne!(first, second);
    }

    #[test]
    fn plan_local_symbol_build_only_reparses_changed_files() {
        let temp_dir = tempfile::tempdir().unwrap_or_else(|error| panic!("temp dir: {error}"));
        let project_root = temp_dir.path();
        std::fs::create_dir_all(project_root.join("src"))
            .unwrap_or_else(|error| panic!("create src: {error}"));
        std::fs::write(project_root.join("src/lib.rs"), "fn alpha() {}\n")
            .unwrap_or_else(|error| panic!("write lib: {error}"));
        std::fs::write(project_root.join("src/extra.rs"), "fn gamma() {}\n")
            .unwrap_or_else(|error| panic!("write extra: {error}"));
        let projects = vec![UiProjectConfig {
            name: "demo".to_string(),
            root: ".".to_string(),
            dirs: vec![".".to_string()],
        }];

        let first =
            plan_local_symbol_build(project_root, project_root, &projects, None, BTreeMap::new());
        assert_eq!(first.base_epoch, None);
        assert_eq!(count_changed_hits(&first), 2);

        std::thread::sleep(Duration::from_millis(5));
        std::fs::write(project_root.join("src/lib.rs"), "fn beta() {}\n")
            .unwrap_or_else(|error| panic!("rewrite lib: {error}"));

        let second = plan_local_symbol_build(
            project_root,
            project_root,
            &projects,
            Some(7),
            first.file_fingerprints.clone(),
        );
        assert_eq!(second.base_epoch, Some(7));
        let changed_partition = only_partition(&second);
        assert_eq!(
            changed_partition.replaced_paths,
            BTreeSet::from(["src/lib.rs".to_string()])
        );
        assert_eq!(changed_partition.changed_hits.len(), 1);
        assert_eq!(changed_partition.changed_hits[0].name, "beta");
    }

    #[tokio::test]
    async fn local_symbol_incremental_refresh_reuses_unchanged_rows() {
        let temp_dir = tempfile::tempdir().unwrap_or_else(|error| panic!("temp dir: {error}"));
        let project_root = temp_dir.path().join("workspace");
        let storage_root = temp_dir.path().join("search_plane");
        std::fs::create_dir_all(project_root.join("src"))
            .unwrap_or_else(|error| panic!("create src: {error}"));
        std::fs::write(project_root.join("src/lib.rs"), "fn alpha() {}\n")
            .unwrap_or_else(|error| panic!("write lib: {error}"));
        std::fs::write(project_root.join("src/extra.rs"), "fn gamma() {}\n")
            .unwrap_or_else(|error| panic!("write extra: {error}"));
        let projects = vec![UiProjectConfig {
            name: "demo".to_string(),
            root: ".".to_string(),
            dirs: vec![".".to_string()],
        }];
        let keyspace =
            SearchManifestKeyspace::new("xiuxian:test:search_plane:local-symbol-incremental");
        let cache = SearchPlaneCache::for_tests(keyspace.clone());
        let service = SearchPlaneService::with_runtime(
            project_root.clone(),
            storage_root,
            keyspace,
            SearchMaintenancePolicy::default(),
            cache,
        );

        super::ensure_local_symbol_index_started(
            &service,
            project_root.as_path(),
            project_root.as_path(),
            &projects,
        );
        wait_for_local_symbol_ready(&service, None).await;

        let initial_gamma = search_local_symbols(&service, "gamma", 10)
            .await
            .unwrap_or_else(|error| panic!("query gamma: {error}"));
        assert_eq!(initial_gamma.len(), 1);
        let initial_alpha = search_local_symbols(&service, "alpha", 10)
            .await
            .unwrap_or_else(|error| panic!("query alpha: {error}"));
        assert_eq!(initial_alpha.len(), 1);

        std::fs::write(project_root.join("src/lib.rs"), "fn beta() {}\n")
            .unwrap_or_else(|error| panic!("rewrite lib: {error}"));
        super::ensure_local_symbol_index_started(
            &service,
            project_root.as_path(),
            project_root.as_path(),
            &projects,
        );
        wait_for_local_symbol_ready(&service, Some(1)).await;

        let gamma = search_local_symbols(&service, "gamma", 10)
            .await
            .unwrap_or_else(|error| panic!("query gamma after refresh: {error}"));
        assert_eq!(gamma.len(), 1);
        let beta = search_local_symbols(&service, "beta", 10)
            .await
            .unwrap_or_else(|error| panic!("query beta after refresh: {error}"));
        assert_eq!(beta.len(), 1);
        let alpha = search_local_symbols(&service, "alpha", 10)
            .await
            .unwrap_or_else(|error| panic!("query alpha after refresh: {error}"));
        assert!(alpha.is_empty());
    }

    #[tokio::test]
    async fn local_symbol_build_writes_partitioned_epoch_tables_for_multiple_scopes() {
        let temp_dir = tempfile::tempdir().unwrap_or_else(|error| panic!("temp dir: {error}"));
        let project_root = temp_dir.path().join("workspace");
        let storage_root = temp_dir.path().join("search_plane");
        std::fs::create_dir_all(project_root.join("packages/alpha/src"))
            .unwrap_or_else(|error| panic!("create alpha: {error}"));
        std::fs::create_dir_all(project_root.join("packages/beta/src"))
            .unwrap_or_else(|error| panic!("create beta: {error}"));
        std::fs::write(
            project_root.join("packages/alpha/src/lib.rs"),
            "fn alpha() {}\n",
        )
        .unwrap_or_else(|error| panic!("write alpha: {error}"));
        std::fs::write(
            project_root.join("packages/beta/src/lib.rs"),
            "fn beta() {}\n",
        )
        .unwrap_or_else(|error| panic!("write beta: {error}"));
        let projects = vec![UiProjectConfig {
            name: "demo".to_string(),
            root: ".".to_string(),
            dirs: vec!["packages/alpha".to_string(), "packages/beta".to_string()],
        }];
        let service = SearchPlaneService::with_paths(
            project_root.clone(),
            storage_root,
            SearchManifestKeyspace::new("xiuxian:test:search_plane:local-symbol-partitioned-build"),
            SearchMaintenancePolicy::default(),
        );

        super::ensure_local_symbol_index_started(
            &service,
            project_root.as_path(),
            project_root.as_path(),
            &projects,
        );
        wait_for_local_symbol_ready(&service, None).await;

        let active_epoch = service
            .coordinator()
            .status_for(SearchCorpusKind::LocalSymbol)
            .active_epoch
            .unwrap_or_default();
        let table_names =
            service.local_epoch_table_names_for_reads(SearchCorpusKind::LocalSymbol, active_epoch);
        assert_eq!(table_names.len(), 2);

        let alpha = search_local_symbols(&service, "alpha", 10)
            .await
            .unwrap_or_else(|error| panic!("query alpha: {error}"));
        assert_eq!(alpha.len(), 1);
        assert_eq!(alpha[0].project_name.as_deref(), Some("demo"));
        assert_eq!(alpha[0].root_label.as_deref(), Some("alpha"));

        let beta = search_local_symbols(&service, "beta", 10)
            .await
            .unwrap_or_else(|error| panic!("query beta: {error}"));
        assert_eq!(beta.len(), 1);
        assert_eq!(beta[0].project_name.as_deref(), Some("demo"));
        assert_eq!(beta[0].root_label.as_deref(), Some("beta"));
    }

    fn count_changed_hits(plan: &super::LocalSymbolBuildPlan) -> usize {
        plan.partitions
            .values()
            .map(|partition| partition.changed_hits.len())
            .sum()
    }

    fn only_partition(plan: &super::LocalSymbolBuildPlan) -> &super::LocalSymbolPartitionBuildPlan {
        assert_eq!(plan.partitions.len(), 1);
        plan.partitions.values().next().expect("single partition")
    }

    async fn wait_for_local_symbol_ready(
        service: &SearchPlaneService,
        previous_epoch: Option<u64>,
    ) {
        for _ in 0..100 {
            let status = service
                .coordinator()
                .status_for(SearchCorpusKind::LocalSymbol);
            if status.phase == crate::search_plane::SearchPlanePhase::Ready
                && status.active_epoch.is_some()
                && previous_epoch
                    .is_none_or(|epoch| status.active_epoch.unwrap_or_default() > epoch)
            {
                return;
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
        panic!("local symbol build did not reach ready state");
    }
}
