use std::sync::Arc;

use xiuxian_zhenfa::ZhenfaSignal;

use crate::analyzers::registry::PluginRegistry;
use crate::gateway::studio::repo_index::RepoIndexCoordinator;
use crate::gateway::studio::router::config::{
    load_ui_config_from_wendao_toml, resolve_studio_config_root,
};
use crate::gateway::studio::router::state::types::{GatewayState, StudioState};
use crate::gateway::studio::symbol_index::SymbolIndexCoordinator;
use crate::gateway::studio::types::UiConfig;
use crate::link_graph::LinkGraphIndex;
use crate::search_plane::SearchPlaneService;
#[cfg(test)]
use crate::search_plane::{SearchMaintenancePolicy, SearchManifestKeyspace};

impl Drop for StudioState {
    fn drop(&mut self) {
        self.stop_background_services();
    }
}

impl GatewayState {
    /// Create gateway state shared by the CLI endpoints and Studio router.
    #[must_use]
    pub fn new(
        index: Option<Arc<LinkGraphIndex>>,
        signal_tx: Option<tokio::sync::mpsc::UnboundedSender<ZhenfaSignal>>,
        plugin_registry: Arc<PluginRegistry>,
    ) -> Self {
        Self {
            index,
            signal_tx,
            studio: Arc::new(StudioState::new_with_bootstrap_ui_config(plugin_registry)),
        }
    }
}

impl StudioState {
    /// Create a new `StudioState` with default configuration.
    #[must_use]
    pub fn new() -> Self {
        Self::new_with_bootstrap_ui_config(Arc::new(PluginRegistry::new()))
    }

    /// Create a new `StudioState` and bootstrap UI config from `wendao.toml`.
    #[must_use]
    pub fn new_with_bootstrap_ui_config(plugin_registry: Arc<PluginRegistry>) -> Self {
        let project_root = xiuxian_io::PrjDirs::project_root();
        let config_root = resolve_studio_config_root(project_root.as_path());
        let search_plane = SearchPlaneService::new(project_root.clone());
        let repo_index = Arc::new(RepoIndexCoordinator::new(
            project_root.clone(),
            Arc::clone(&plugin_registry),
            search_plane.clone(),
        ));
        let symbol_index_coordinator = Arc::new(SymbolIndexCoordinator::new(
            project_root.clone(),
            config_root.clone(),
        ));
        let state = Self {
            project_root,
            config_root,
            ui_config: Arc::new(std::sync::RwLock::new(UiConfig {
                projects: Vec::new(),
                repo_projects: Vec::new(),
            })),
            graph_index: Arc::new(std::sync::RwLock::new(None)),
            symbol_index: Arc::new(std::sync::RwLock::new(None)),
            symbol_index_coordinator,
            search_plane,
            vfs_scan: Arc::new(std::sync::RwLock::new(None)),
            repo_index,
            plugin_registry,
        };
        state.repo_index.start();
        if let Some(config) = load_ui_config_from_wendao_toml(state.config_root.as_path()) {
            state.set_ui_config(config);
        }
        state
    }

    #[cfg(test)]
    #[must_use]
    pub(crate) fn new_with_bootstrap_ui_config_and_search_plane_root(
        plugin_registry: Arc<PluginRegistry>,
        search_plane_root: std::path::PathBuf,
    ) -> Self {
        let project_root = xiuxian_io::PrjDirs::project_root();
        let config_root = resolve_studio_config_root(project_root.as_path());
        let manifest_keyspace = SearchManifestKeyspace::new(format!(
            "xiuxian:test:search_plane:{}",
            blake3::hash(search_plane_root.to_string_lossy().as_bytes()).to_hex()
        ));
        let search_plane = SearchPlaneService::with_paths(
            project_root.clone(),
            search_plane_root,
            manifest_keyspace,
            SearchMaintenancePolicy::default(),
        );
        let repo_index = Arc::new(RepoIndexCoordinator::new(
            project_root.clone(),
            Arc::clone(&plugin_registry),
            search_plane.clone(),
        ));
        let symbol_index_coordinator = Arc::new(SymbolIndexCoordinator::new(
            project_root.clone(),
            config_root.clone(),
        ));
        let state = Self {
            project_root,
            config_root,
            ui_config: Arc::new(std::sync::RwLock::new(UiConfig {
                projects: Vec::new(),
                repo_projects: Vec::new(),
            })),
            graph_index: Arc::new(std::sync::RwLock::new(None)),
            symbol_index: Arc::new(std::sync::RwLock::new(None)),
            symbol_index_coordinator,
            search_plane,
            vfs_scan: Arc::new(std::sync::RwLock::new(None)),
            repo_index,
            plugin_registry,
        };
        state.repo_index.start();
        state
    }

    pub(crate) fn stop_background_services(&self) {
        self.repo_index.stop();
        self.symbol_index_coordinator.stop();
        self.search_plane.stop_background_maintenance();
    }
}
