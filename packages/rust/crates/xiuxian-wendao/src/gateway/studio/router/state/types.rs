use std::path::PathBuf;
use std::sync::{Arc, RwLock};

use crate::analyzers::registry::PluginRegistry;
use crate::gateway::studio::repo_index::RepoIndexCoordinator;
use crate::gateway::studio::symbol_index::SymbolIndexCoordinator;
use crate::gateway::studio::types::UiConfig;
use crate::link_graph::LinkGraphIndex;
use crate::search_plane::SearchPlaneService;
use crate::unified_symbol::UnifiedSymbolIndex;

use crate::gateway::studio::types::VfsScanResult;
use xiuxian_zhenfa::ZhenfaSignal;

/// Shared state for the Studio API.
///
/// Contains configuration, VFS roots, and cached graph index.
pub struct StudioState {
    pub(crate) project_root: PathBuf,
    pub(crate) config_root: PathBuf,
    pub(crate) ui_config: Arc<RwLock<UiConfig>>,
    pub(crate) graph_index: Arc<RwLock<Option<Arc<LinkGraphIndex>>>>,
    pub(crate) symbol_index: Arc<RwLock<Option<Arc<UnifiedSymbolIndex>>>>,
    pub(crate) symbol_index_coordinator: Arc<SymbolIndexCoordinator>,
    pub(crate) search_plane: SearchPlaneService,
    pub(crate) vfs_scan: Arc<RwLock<Option<VfsScanResult>>>,
    pub(crate) repo_index: Arc<RepoIndexCoordinator>,
    /// Registry of repository intelligence plugins.
    pub(crate) plugin_registry: Arc<PluginRegistry>,
}

/// Shared state used by the top-level gateway process.
#[derive(Clone)]
pub struct GatewayState {
    /// Optional graph index for CLI-powered stats endpoint.
    pub index: Option<Arc<LinkGraphIndex>>,
    /// Signal sender for notification worker.
    pub signal_tx: Option<tokio::sync::mpsc::UnboundedSender<ZhenfaSignal>>,
    /// Studio-specific state for VFS/graph/search APIs.
    pub studio: Arc<StudioState>,
}
