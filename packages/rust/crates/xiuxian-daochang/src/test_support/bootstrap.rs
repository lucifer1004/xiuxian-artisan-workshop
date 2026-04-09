//! Bootstrap helper exports for integration tests.

use std::path::{Path, PathBuf};

use crate::agent::bootstrap::qianhuan::init_persona_registries;
use crate::agent::bootstrap::service_mount::ServiceMountCatalog;
use crate::agent::bootstrap::zhixing::{
    load_skill_templates_from_embedded_registry as load_skill_templates_from_embedded_registry_internal,
    resolve_notebook_root as resolve_notebook_root_internal,
    resolve_prj_data_home_with_env as resolve_prj_data_home_with_env_internal,
    resolve_project_root_with_prj_root as resolve_project_root_with_prj_root_internal,
    resolve_template_globs_with_resource_root as resolve_template_globs_with_resource_root_internal,
};
use crate::config::XiuxianConfig;
use anyhow::Result;
use xiuxian_qianhuan::ManifestationManager;

/// Test-facing summary for embedded skill template loading.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BootstrapTemplateLoadSummary {
    /// Count of semantic wendao URI links attached to the manager.
    pub linked_ids: usize,
    /// Count of embedded template records loaded into the bridge.
    pub template_records: usize,
    /// Count of template names that became renderable after loading.
    pub loaded_template_names: usize,
}

/// Resolve project root with an explicit `PRJ_ROOT` override for tests.
#[must_use]
pub fn resolve_project_root(prj_root: Option<&str>, current_dir: &Path) -> PathBuf {
    resolve_project_root_with_prj_root_internal(prj_root, current_dir)
}

/// Resolve `PRJ_DATA_HOME` with an optional explicit override.
#[must_use]
pub fn resolve_prj_data_home(project_root: &Path, prj_data_home: Option<&str>) -> PathBuf {
    resolve_prj_data_home_with_env_internal(project_root, prj_data_home)
}

/// Resolve notebook root with env-over-config-over-default precedence.
#[must_use]
pub fn resolve_notebook_root(
    prj_data_home: &Path,
    env_notebook_path: Option<&str>,
    config_notebook_path: Option<&str>,
) -> PathBuf {
    resolve_notebook_root_internal(
        prj_data_home,
        env_notebook_path.map(str::to_owned),
        config_notebook_path.map(str::to_owned),
    )
}

/// Resolve template globs with an optional resource-root override.
#[must_use]
pub fn resolve_template_globs(
    project_root: &Path,
    config_template_paths: Option<Vec<String>>,
    resource_root_override: Option<&str>,
) -> Vec<String> {
    resolve_template_globs_with_resource_root_internal(
        project_root,
        config_template_paths,
        resource_root_override,
    )
}

/// Load embedded skill templates into a manifestation manager.
///
/// # Errors
///
/// Returns an error when embedded template loading fails.
pub fn load_skill_templates_from_embedded_registry(
    manager: &ManifestationManager,
) -> Result<BootstrapTemplateLoadSummary> {
    load_skill_templates_from_embedded_registry_internal(manager)
        .map(|summary| BootstrapTemplateLoadSummary {
            linked_ids: summary.linked_ids,
            template_records: summary.template_records,
            loaded_template_names: summary.loaded_template_names,
        })
        .map_err(anyhow::Error::msg)
}

/// Return the internal persona-registry size produced by bootstrap wiring.
#[must_use]
pub fn init_persona_registries_internal_len(
    project_root: &Path,
    xiuxian_cfg: &XiuxianConfig,
) -> usize {
    let mut mounts = ServiceMountCatalog::new();
    init_persona_registries(project_root, xiuxian_cfg, &mut mounts).internal_len()
}
