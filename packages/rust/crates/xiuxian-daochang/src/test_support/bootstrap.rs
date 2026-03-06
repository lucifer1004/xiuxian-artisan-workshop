//! Bootstrap helpers exposed for integration tests.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use xiuxian_qianhuan::ManifestationManager;
use xiuxian_wendao::{IncrementalSyncPolicy, SkillVfsResolver};

use crate::agent::ServiceMountRecord;
use crate::agent::bootstrap::{hot_reload, memory, qianhuan, service_mount, zhenfa, zhixing};
use crate::config::{MemoryConfig, RuntimeSettings, XiuxianConfig};

pub use service_mount::ServiceMountStatus;

/// Test-facing mount catalog wrapper for bootstrap assertions.
#[derive(Debug, Default)]
pub struct BootstrapServiceMountCatalog {
    inner: service_mount::ServiceMountCatalog,
}

impl BootstrapServiceMountCatalog {
    #[must_use]
    pub fn new() -> Self {
        Self {
            inner: service_mount::ServiceMountCatalog::new(),
        }
    }

    #[must_use]
    pub fn finish(self) -> Vec<ServiceMountRecord> {
        self.inner.finish()
    }

    pub(crate) fn inner_mut(&mut self) -> &mut service_mount::ServiceMountCatalog {
        &mut self.inner
    }
}

/// Minimal summary for embedded skill-template bridge load assertions.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct SkillTemplateLoadSummary {
    pub linked_ids: usize,
    pub template_records: usize,
    pub loaded_template_names: usize,
}

#[must_use]
pub fn resolve_project_root_with_prj_root(prj_root: Option<&str>, current_dir: &Path) -> PathBuf {
    zhixing::resolve_project_root_with_prj_root(prj_root, current_dir)
}

#[must_use]
pub fn resolve_prj_data_home_with_env(project_root: &Path, prj_data_home: Option<&str>) -> PathBuf {
    zhixing::resolve_prj_data_home_with_env(project_root, prj_data_home)
}

#[must_use]
pub fn resolve_notebook_root(
    prj_data_home: &Path,
    env_notebook_path: Option<String>,
    config_notebook_path: Option<String>,
) -> PathBuf {
    zhixing::resolve_notebook_root(prj_data_home, env_notebook_path, config_notebook_path)
}

#[must_use]
pub fn resolve_template_globs(
    project_root: &Path,
    config_template_paths: Option<Vec<String>>,
) -> Vec<String> {
    zhixing::resolve_template_globs(project_root, config_template_paths)
}

#[must_use]
pub fn resolve_template_globs_with_resource_root(
    project_root: &Path,
    config_template_paths: Option<Vec<String>>,
    resource_root_override: Option<&str>,
) -> Vec<String> {
    zhixing::resolve_template_globs_with_resource_root(
        project_root,
        config_template_paths,
        resource_root_override,
    )
}

/// Load linked skill templates from embedded semantic resources.
///
/// # Errors
///
/// Returns an error when embedded resource discovery or manifestation loading fails.
pub fn load_skill_templates_from_embedded_registry(
    manager: &ManifestationManager,
) -> Result<SkillTemplateLoadSummary, String> {
    zhixing::load_skill_templates_from_embedded_registry(manager).map(|summary| {
        SkillTemplateLoadSummary {
            linked_ids: summary.linked_ids,
            template_records: summary.template_records,
            loaded_template_names: summary.loaded_template_names,
        }
    })
}

#[must_use]
pub fn resolve_wendao_incremental_policy(
    patterns: &[String],
    configured_extensions: Option<&[String]>,
) -> IncrementalSyncPolicy {
    hot_reload::resolve_wendao_incremental_policy(patterns, configured_extensions)
}

#[must_use]
pub fn resolve_wendao_watch_patterns(
    configured_patterns: Option<&[String]>,
    configured_extensions: Option<&[String]>,
) -> Vec<String> {
    hot_reload::resolve_wendao_watch_patterns(configured_patterns, configured_extensions)
}

#[must_use]
pub fn resolve_wendao_watch_roots(
    project_root: &Path,
    default_notebook_root: &Path,
    watch_dirs: Option<&Vec<String>>,
    include_dirs: Option<&Vec<String>>,
) -> Vec<PathBuf> {
    hot_reload::resolve_wendao_watch_roots(
        project_root,
        default_notebook_root,
        watch_dirs,
        include_dirs,
    )
}

#[must_use]
pub fn resolve_memory_embed_base_url(
    memory_cfg: &MemoryConfig,
    runtime_settings: &RuntimeSettings,
) -> String {
    memory::resolve_memory_embed_base_url(memory_cfg, runtime_settings)
}

#[must_use]
pub fn resolve_memory_embedding_backend_hint_with_inputs(
    env_memory_backend: Option<&str>,
    env_embed_backend: Option<&str>,
    memory_backend: Option<&str>,
    runtime_memory_backend: Option<&str>,
    runtime_embed_backend: Option<&str>,
) -> Option<String> {
    memory::resolve_memory_embedding_backend_hint_with_for_tests(
        env_memory_backend,
        env_embed_backend,
        memory_backend,
        runtime_memory_backend,
        runtime_embed_backend,
    )
}

#[must_use]
pub fn init_persona_registries_internal_len(
    project_root: &Path,
    xiuxian_cfg: &XiuxianConfig,
    mounts: &mut BootstrapServiceMountCatalog,
) -> usize {
    qianhuan::init_persona_registries(project_root, xiuxian_cfg, mounts.inner_mut()).internal_len()
}

#[must_use]
pub fn build_skill_vfs_resolver_from_roots(
    roots: &[PathBuf],
    mounts: &mut BootstrapServiceMountCatalog,
) -> Option<Arc<SkillVfsResolver>> {
    zhenfa::build_skill_vfs_resolver_from_roots(roots, mounts.inner_mut())
}
