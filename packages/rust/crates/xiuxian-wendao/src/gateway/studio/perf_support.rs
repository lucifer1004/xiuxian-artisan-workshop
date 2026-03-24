//! Performance-test support for Studio gateway warm-cache scenarios.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};

use anyhow::{Result, anyhow};
use axum::Router;
use git2::{IndexAddOption, Repository, Signature, Time};

use crate::analyzers::{
    analyze_registered_repository_with_registry, bootstrap_builtin_registry,
    load_repo_intelligence_config,
};
use crate::search_plane::SearchPlaneService;

use super::repo_index::{RepoCodeDocument, RepoIndexCoordinator};
use super::router::{GatewayState, StudioState, load_ui_config_from_wendao_toml, studio_router};
use super::symbol_index::SymbolIndexCoordinator;

/// Prepared Studio gateway fixture for performance tests.
pub struct GatewayPerfFixture {
    root: PathBuf,
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
        &self.root
    }
}

impl Drop for GatewayPerfFixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
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
    Ok(GatewayPerfFixture { root, state })
}

fn create_perf_root() -> Result<PathBuf> {
    let root = std::env::temp_dir().join(format!(
        "xiuxian-wendao-gateway-perf-{}",
        uuid::Uuid::new_v4()
    ));
    fs::create_dir_all(&root)?;
    Ok(root)
}

fn gateway_state_for_project(project_root: &Path) -> Result<Arc<GatewayState>> {
    let config_root = project_root.to_path_buf();
    let ui_config = load_ui_config_from_wendao_toml(&config_root).unwrap_or_default();
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
        .publish_repo_entities_with_revision(repo_id, &analysis, None)
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
