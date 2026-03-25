//! Performance-test support for Studio gateway warm-cache scenarios.

use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};
use std::time::Duration;

use anyhow::{Result, anyhow};
use axum::Router;
use git2::{IndexAddOption, Repository, Signature, Time};
use tokio::time::{Instant, sleep};

use crate::analyzers::{
    RegisteredRepository, RepositoryPluginConfig, RepositoryRef, RepositoryRefreshPolicy,
    analyze_registered_repository_with_registry, bootstrap_builtin_registry,
    load_repo_intelligence_config,
};
use crate::search_plane::SearchPlaneService;

use super::repo_index::RepoIndexStatusResponse;
use super::repo_index::{RepoCodeDocument, RepoIndexCoordinator};
use super::router::{
    GatewayState, StudioState, configured_repositories, load_ui_config_from_wendao_toml,
    studio_router,
};
use super::symbol_index::SymbolIndexCoordinator;
use super::types::{UiConfig, UiRepoProjectConfig};

const REAL_WORKSPACE_ROOT_ENV: &str = "XIUXIAN_WENDAO_GATEWAY_PERF_WORKSPACE_ROOT";
const REAL_WORKSPACE_READY_TIMEOUT_ENV: &str = "XIUXIAN_WENDAO_GATEWAY_PERF_READY_TIMEOUT_SECS";
const DEFAULT_REAL_WORKSPACE_ROOT: &str = ".data/wendao-frontend";
const DEFAULT_REAL_WORKSPACE_READY_TIMEOUT_SECS: u64 = 900;

enum GatewayPerfRoot {
    Owned(PathBuf),
    External(PathBuf),
}

/// Prepared Studio gateway fixture for performance tests.
pub struct GatewayPerfFixture {
    root: GatewayPerfRoot,
    state: Arc<GatewayState>,
}

impl GatewayPerfFixture {
    /// Return a fresh Studio router bound to the prepared gateway state.
    pub fn router(&self) -> Router {
        studio_router(Arc::clone(&self.state))
    }

    /// Return the shared gateway state backing this fixture.
    #[must_use]
    pub fn state(&self) -> Arc<GatewayState> {
        Arc::clone(&self.state)
    }

    /// Return the temporary project root backing this fixture.
    #[must_use]
    pub fn root(&self) -> &Path {
        match &self.root {
            GatewayPerfRoot::Owned(path) | GatewayPerfRoot::External(path) => path.as_path(),
        }
    }

    /// Execute one direct repo-scoped search-plane query so status telemetry
    /// exposes a repo-specific scope bucket without paying for the full HTTP
    /// search handler chain.
    ///
    /// # Errors
    ///
    /// Returns an error if the repo-backed search-plane query fails or does
    /// not return any hits for the requested repo/query pair.
    pub async fn warm_repo_scope_query(&self, repo_id: &str, query: &str) -> Result<()> {
        let hits = self
            .state
            .studio
            .search_plane
            .search_repo_entities(repo_id, query, &HashSet::new(), &HashSet::new(), 5)
            .await
            .map_err(|error| anyhow!("failed to warm repo-scoped search telemetry: {error}"))?;
        if hits.is_empty() {
            return Err(anyhow!(
                "repo-scoped search warmup returned no hits for repo `{repo_id}` and query `{query}`"
            ));
        }
        Ok(())
    }
}

impl Drop for GatewayPerfFixture {
    fn drop(&mut self) {
        self.state.studio.stop_background_services();
        if let GatewayPerfRoot::Owned(path) = &self.root {
            let _ = fs::remove_dir_all(path);
        }
    }
}

/// Build a warm-cache gateway fixture with one Julia repository.
///
/// # Errors
///
/// Returns an error if the temporary project cannot be created, initialized as
/// a Git repository, analyzed, or published into the search plane.
pub async fn prepare_gateway_perf_fixture() -> Result<GatewayPerfFixture> {
    let root = create_perf_root()?;
    let repo_dir = create_local_git_repo(root.as_path(), "GatewaySyncPkg")?;
    write_default_repo_config(root.as_path(), repo_dir.as_path(), "gateway-sync")?;
    let state = gateway_state_for_project(root.as_path())?;
    publish_code_search_snapshot(&state, "gateway-sync").await?;
    Ok(GatewayPerfFixture {
        root: GatewayPerfRoot::Owned(root),
        state,
    })
}

/// Build a warm-cache gateway fixture backed by a real multi-repository
/// workspace.
///
/// The workspace root is resolved from
/// `XIUXIAN_WENDAO_GATEWAY_PERF_WORKSPACE_ROOT` first, then falls back to
/// `.data/wendao-frontend` under the current project root when present.
///
/// # Errors
///
/// Returns an error when no real workspace root can be resolved, when the
/// target workspace cannot bootstrap gateway state, or when repo indexing does
/// not reach a query-ready state before the configured timeout.
pub async fn prepare_gateway_real_workspace_perf_fixture() -> Result<GatewayPerfFixture> {
    let root = resolve_real_workspace_root().ok_or_else(|| {
        anyhow!(
            "real gateway perf workspace root is not available; set {REAL_WORKSPACE_ROOT_ENV} or create {DEFAULT_REAL_WORKSPACE_ROOT}"
        )
    })?;
    let state = gateway_state_for_project(root.as_path())?;
    warm_real_workspace_search_plane(&state).await?;
    Ok(GatewayPerfFixture {
        root: GatewayPerfRoot::External(root),
        state,
    })
}

fn create_perf_root() -> Result<PathBuf> {
    let root = std::env::temp_dir().join(format!(
        "xiuxian-wendao-gateway-perf-{}",
        uuid::Uuid::new_v4()
    ));
    fs::create_dir_all(&root)?;
    Ok(root)
}

fn resolve_real_workspace_root() -> Option<PathBuf> {
    let project_root = xiuxian_io::PrjDirs::project_root();
    resolve_real_workspace_root_with_lookup(project_root.as_path(), &|key| std::env::var(key).ok())
}

fn resolve_real_workspace_root_with_lookup(
    project_root: &Path,
    lookup: &dyn Fn(&str) -> Option<String>,
) -> Option<PathBuf> {
    if let Some(path) = lookup(REAL_WORKSPACE_ROOT_ENV)
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
    {
        let path = PathBuf::from(path);
        let resolved = if path.is_absolute() {
            path
        } else {
            project_root.join(path)
        };
        return Some(resolved);
    }

    let fallback = project_root.join(DEFAULT_REAL_WORKSPACE_ROOT);
    fallback.exists().then_some(fallback)
}

fn real_workspace_ready_timeout() -> Duration {
    let parsed = std::env::var(REAL_WORKSPACE_READY_TIMEOUT_ENV)
        .ok()
        .and_then(|raw| raw.trim().parse::<u64>().ok())
        .filter(|value| *value > 0);
    Duration::from_secs(parsed.unwrap_or(DEFAULT_REAL_WORKSPACE_READY_TIMEOUT_SECS))
}

fn gateway_state_for_project(project_root: &Path) -> Result<Arc<GatewayState>> {
    let config_root = project_root.to_path_buf();
    let ui_config = gateway_ui_config_for_project(config_root.as_path())?;
    let plugin_registry = Arc::new(bootstrap_builtin_registry()?);
    let search_plane = SearchPlaneService::new(project_root.to_path_buf());
    let repo_index = Arc::new(RepoIndexCoordinator::new(
        project_root.to_path_buf(),
        Arc::clone(&plugin_registry),
        search_plane.clone(),
    ));
    repo_index.start();

    let state = Arc::new(GatewayState {
        index: None,
        signal_tx: None,
        studio: Arc::new(StudioState {
            project_root: project_root.to_path_buf(),
            config_root: config_root.clone(),
            ui_config: Arc::new(RwLock::new(ui_config)),
            graph_index: Arc::new(RwLock::new(None)),
            symbol_index: Arc::new(RwLock::new(None)),
            symbol_index_coordinator: Arc::new(SymbolIndexCoordinator::new(
                project_root.to_path_buf(),
                config_root.clone(),
            )),
            search_plane,
            vfs_scan: Arc::new(RwLock::new(None)),
            repo_index,
            plugin_registry,
        }),
    });

    Ok(state)
}

fn gateway_ui_config_for_project(config_root: &Path) -> Result<UiConfig> {
    let mut ui_config = load_ui_config_from_wendao_toml(config_root).unwrap_or_default();
    if !ui_config.repo_projects.is_empty() {
        return Ok(ui_config);
    }

    let config = load_repo_intelligence_config(
        Some(config_root.join("wendao.toml").as_path()),
        config_root,
    )?;
    ui_config.repo_projects = config
        .repos
        .into_iter()
        .map(ui_repo_project_from_registered_repository)
        .collect();
    Ok(ui_config)
}

fn ui_repo_project_from_registered_repository(
    repository: RegisteredRepository,
) -> UiRepoProjectConfig {
    UiRepoProjectConfig {
        id: repository.id,
        root: repository
            .path
            .map(|path| path.to_string_lossy().into_owned()),
        url: repository.url,
        git_ref: repository.git_ref.as_ref().map(repository_ref_string),
        refresh: Some(repository_refresh_policy_string(repository.refresh).to_string()),
        plugins: repository
            .plugins
            .into_iter()
            .map(|plugin| match plugin {
                RepositoryPluginConfig::Id(id) => id,
                RepositoryPluginConfig::Config { id, .. } => id,
            })
            .collect(),
    }
}

fn repository_ref_string(reference: &RepositoryRef) -> String {
    reference.as_str().to_string()
}

fn repository_refresh_policy_string(refresh: RepositoryRefreshPolicy) -> &'static str {
    match refresh {
        RepositoryRefreshPolicy::Fetch => "fetch",
        RepositoryRefreshPolicy::Manual => "manual",
    }
}

async fn warm_real_workspace_search_plane(state: &Arc<GatewayState>) -> Result<()> {
    let repositories = configured_repositories(&state.studio);
    if repositories.is_empty() {
        return Err(anyhow!(
            "real workspace fixture requires at least one configured repository"
        ));
    }

    state
        .studio
        .repo_index
        .ensure_repositories_enqueued(repositories.clone(), false);
    wait_for_repo_index_ready(state, repositories.len()).await
}

async fn wait_for_repo_index_ready(
    state: &Arc<GatewayState>,
    expected_repositories: usize,
) -> Result<()> {
    let timeout = real_workspace_ready_timeout();
    let start = Instant::now();
    loop {
        let status = state.studio.repo_index.status_response(None);
        if real_workspace_status_is_query_ready(&status, expected_repositories) {
            return Ok(());
        }

        if start.elapsed() >= timeout {
            return Err(anyhow!(
                "timed out waiting for repo index bootstrap after {:?} (total={}, ready={}, unsupported={}, failed={}, active={}, queued={}, checking={}, syncing={}, indexing={})",
                timeout,
                status.total,
                status.ready,
                status.unsupported,
                status.failed,
                status.active,
                status.queued,
                status.checking,
                status.syncing,
                status.indexing
            ));
        }

        sleep(Duration::from_secs(1)).await;
    }
}

fn real_workspace_status_is_query_ready(
    status: &RepoIndexStatusResponse,
    expected_repositories: usize,
) -> bool {
    status.total >= expected_repositories && status.ready > 0
}

async fn publish_code_search_snapshot(state: &Arc<GatewayState>, repo_id: &str) -> Result<()> {
    let config_path = state.studio.config_root.join("wendao.toml");
    let config = load_repo_intelligence_config(
        Some(config_path.as_path()),
        state.studio.config_root.as_path(),
    )?;
    let repository = config
        .repos
        .iter()
        .find(|repository| repository.id == repo_id)
        .ok_or_else(|| anyhow!("repository `{repo_id}` not found in perf config"))?;
    let analysis = analyze_registered_repository_with_registry(
        repository,
        state.studio.config_root.as_path(),
        &state.studio.plugin_registry,
    )?;

    state
        .studio
        .search_plane
        .publish_repo_entities_with_revision(
            repo_id,
            &analysis,
            &[
                RepoCodeDocument {
                    path: "src/GatewaySyncPkg.jl".to_string(),
                    language: Some("julia".to_string()),
                    contents: Arc::<str>::from(
                        "module GatewaySyncPkg\nexport solve\nsolve() = nothing\nend\n",
                    ),
                    size_bytes: 56,
                    modified_unix_ms: 0,
                },
                RepoCodeDocument {
                    path: "examples/solve_demo.jl".to_string(),
                    language: Some("julia".to_string()),
                    contents: Arc::<str>::from("using GatewaySyncPkg\nsolve()\n"),
                    size_bytes: 29,
                    modified_unix_ms: 0,
                },
            ],
            None,
        )
        .await?;
    state
        .studio
        .search_plane
        .publish_repo_content_chunks_with_revision(
            repo_id,
            &[RepoCodeDocument {
                path: "src/GatewaySyncPkg.jl".to_string(),
                language: Some("julia".to_string()),
                contents: Arc::<str>::from(
                    "module GatewaySyncPkg\nexport solve\nsolve() = nothing\nend\n",
                ),
                size_bytes: 56,
                modified_unix_ms: 0,
            }],
            None,
        )
        .await?;
    Ok(())
}

fn write_default_repo_config(base: &Path, repo_dir: &Path, repo_id: &str) -> Result<()> {
    fs::write(
        base.join("wendao.toml"),
        format!(
            r#"[link_graph.projects.{repo_id}]
root = "{}"
plugins = ["julia"]
"#,
            repo_dir.display()
        ),
    )?;
    Ok(())
}

fn create_local_git_repo(base: &Path, package_name: &str) -> Result<PathBuf> {
    let repo_dir = base.join(package_name.to_ascii_lowercase());
    fs::create_dir_all(repo_dir.join("src"))?;
    fs::write(repo_dir.join("README.md"), "# Gateway Repo\n")?;
    fs::write(
        repo_dir.join("Project.toml"),
        format!(
            r#"name = "{package_name}"
uuid = "12345678-1234-1234-1234-123456789abc"
version = "0.1.0"
"#
        ),
    )?;
    fs::write(
        repo_dir.join("src").join(format!("{package_name}.jl")),
        format!(
            "module {package_name}\nexport solve\n\"\"\"solve docs\"\"\"\nsolve() = nothing\nend\n"
        ),
    )?;
    fs::create_dir_all(repo_dir.join("examples"))?;
    fs::write(
        repo_dir.join("examples").join("solve_demo.jl"),
        format!("using {package_name}\nsolve()\n"),
    )?;
    fs::create_dir_all(repo_dir.join("docs"))?;
    fs::write(repo_dir.join("docs").join("solve.md"), "# solve\n")?;

    let repository = Repository::init(&repo_dir)?;
    repository.remote(
        "origin",
        &format!(
            "https://example.invalid/xiuxian-wendao/{}.git",
            package_name.to_ascii_lowercase()
        ),
    )?;
    commit_all(&repository, "initial import")?;
    Ok(repo_dir)
}

fn commit_all(repository: &Repository, message: &str) -> Result<()> {
    let mut index = repository.index()?;
    index.add_all(["*"], IndexAddOption::DEFAULT, None)?;
    index.write()?;
    let tree_id = index.write_tree()?;
    let tree = repository.find_tree(tree_id)?;
    let signature = Signature::new("Gateway Perf", "gateway-perf@example.invalid", &git_time())?;
    let parent = repository
        .head()
        .ok()
        .and_then(|head| head.target())
        .and_then(|target| repository.find_commit(target).ok());

    match parent {
        Some(ref commit) => {
            repository.commit(
                Some("HEAD"),
                &signature,
                &signature,
                message,
                &tree,
                &[commit],
            )?;
        }
        None => {
            repository.commit(Some("HEAD"), &signature, &signature, message, &tree, &[])?;
        }
    }

    Ok(())
}

fn git_time() -> Time {
    let seconds = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_else(|error| panic!("system time before unix epoch: {error}"))
        .as_secs();
    let seconds = i64::try_from(seconds).unwrap_or(i64::MAX);
    Time::new(seconds, 0)
}

#[cfg(test)]
mod tests {
    use super::{
        DEFAULT_REAL_WORKSPACE_ROOT, GatewayPerfRoot, REAL_WORKSPACE_ROOT_ENV,
        real_workspace_status_is_query_ready, resolve_real_workspace_root_with_lookup,
    };
    use crate::gateway::studio::repo_index::RepoIndexStatusResponse;
    use std::fs;
    use std::path::{Path, PathBuf};

    #[test]
    fn resolve_real_workspace_root_prefers_explicit_env_override() {
        let resolved = resolve_real_workspace_root_with_lookup(Path::new("/tmp/project"), &|key| {
            (key == REAL_WORKSPACE_ROOT_ENV).then_some("/tmp/custom-workspace".to_string())
        });
        assert_eq!(resolved, Some(PathBuf::from("/tmp/custom-workspace")));
    }

    #[test]
    fn resolve_real_workspace_root_resolves_relative_override_from_project_root() {
        let resolved = resolve_real_workspace_root_with_lookup(Path::new("/tmp/project"), &|key| {
            (key == REAL_WORKSPACE_ROOT_ENV).then_some(".data/wendao-frontend".to_string())
        });
        assert_eq!(
            resolved,
            Some(PathBuf::from("/tmp/project/.data/wendao-frontend"))
        );
    }

    #[test]
    fn resolve_real_workspace_root_uses_default_frontend_workspace_when_present() {
        let root = std::env::temp_dir().join(format!(
            "xiuxian-wendao-perf-support-{}",
            uuid::Uuid::new_v4()
        ));
        let fallback = root.join(DEFAULT_REAL_WORKSPACE_ROOT);
        fs::create_dir_all(&fallback)
            .unwrap_or_else(|error| panic!("failed to create fallback workspace root: {error}"));

        let resolved = resolve_real_workspace_root_with_lookup(root.as_path(), &|_| None);
        assert_eq!(resolved, Some(fallback.clone()));

        fs::remove_dir_all(root).unwrap_or_else(|error| {
            panic!("failed to remove temporary perf support root: {error}")
        });
    }

    #[test]
    fn gateway_perf_root_preserves_external_paths() {
        let path = PathBuf::from("/tmp/external-workspace");
        let root = GatewayPerfRoot::External(path.clone());
        let resolved = match root {
            GatewayPerfRoot::Owned(inner) | GatewayPerfRoot::External(inner) => inner,
        };
        assert_eq!(resolved, path);
    }

    #[test]
    fn gateway_ui_config_falls_back_to_repo_intelligence_projects() {
        let root = std::env::temp_dir().join(format!(
            "xiuxian-wendao-perf-ui-config-{}",
            uuid::Uuid::new_v4()
        ));
        fs::create_dir_all(&root)
            .unwrap_or_else(|error| panic!("failed to create temporary config root: {error}"));
        fs::write(
            root.join("wendao.toml"),
            r#"[link_graph.projects."ADTypes.jl"]
url = "https://github.com/SciML/ADTypes.jl.git"
refresh = "fetch"
plugins = ["julia"]
"#,
        )
        .unwrap_or_else(|error| panic!("failed to write temporary wendao.toml: {error}"));

        let ui_config = super::gateway_ui_config_for_project(root.as_path())
            .unwrap_or_else(|error| panic!("failed to build fallback ui config: {error}"));
        assert_eq!(ui_config.repo_projects.len(), 1);
        assert_eq!(ui_config.repo_projects[0].id, "ADTypes.jl");
        assert_eq!(
            ui_config.repo_projects[0].url.as_deref(),
            Some("https://github.com/SciML/ADTypes.jl.git")
        );

        fs::remove_dir_all(root)
            .unwrap_or_else(|error| panic!("failed to remove temporary config root: {error}"));
    }

    #[test]
    fn real_workspace_status_is_query_ready_requires_discovered_repositories() {
        let status = RepoIndexStatusResponse {
            total: 149,
            active: 12,
            queued: 11,
            checking: 0,
            syncing: 4,
            indexing: 8,
            ready: 9,
            unsupported: 0,
            failed: 0,
            target_concurrency: 1,
            max_concurrency: 4,
            sync_concurrency_limit: 1,
            current_repo_id: None,
            active_repo_ids: Vec::new(),
            repos: Vec::new(),
        };
        assert!(!real_workspace_status_is_query_ready(&status, 150));
    }

    #[test]
    fn real_workspace_status_is_query_ready_accepts_partial_active_bootstrap() {
        let status = RepoIndexStatusResponse {
            total: 179,
            active: 57,
            queued: 23,
            checking: 4,
            syncing: 16,
            indexing: 37,
            ready: 12,
            unsupported: 1,
            failed: 0,
            target_concurrency: 1,
            max_concurrency: 4,
            sync_concurrency_limit: 1,
            current_repo_id: None,
            active_repo_ids: Vec::new(),
            repos: Vec::new(),
        };
        assert!(real_workspace_status_is_query_ready(&status, 150));
    }
}
