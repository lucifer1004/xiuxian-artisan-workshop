use std::path::Path;

use xiuxian_wendao_core::repo_intelligence::{
    AnalysisContext, PluginAnalysisOutput, PluginLinkContext, PluginRegistry, RegisteredRepository,
    RelationRecord, RepoIntelligenceError, RepoIntelligencePlugin, RepoSourceFile,
    RepositoryAnalysisOutput,
};

use super::analysis;

const MODELICA_PLUGIN_ID: &str = "modelica";

/// External Modelica analyzer for Repo Intelligence.
#[derive(Debug, Default, Clone, Copy)]
pub struct ModelicaRepoIntelligencePlugin;

/// Register the Modelica plugin into an existing Repo Intelligence registry.
///
/// # Errors
///
/// Returns [`RepoIntelligenceError`] when the registry already contains a
/// plugin with the `modelica` identifier.
pub fn register_modelica_into(registry: &mut PluginRegistry) -> Result<(), RepoIntelligenceError> {
    registry.register(ModelicaRepoIntelligencePlugin)
}

inventory::submit! {
    xiuxian_wendao_core::repo_intelligence::BuiltinPluginRegistrar::new(
        MODELICA_PLUGIN_ID,
        register_modelica_into,
    )
}

impl RepoIntelligencePlugin for ModelicaRepoIntelligencePlugin {
    fn id(&self) -> &'static str {
        MODELICA_PLUGIN_ID
    }

    fn supports_repository(&self, repository: &RegisteredRepository) -> bool {
        repository
            .plugins
            .iter()
            .any(|plugin| plugin.id() == MODELICA_PLUGIN_ID)
    }

    fn analyze_file(
        &self,
        _context: &AnalysisContext,
        _file: &RepoSourceFile,
    ) -> Result<PluginAnalysisOutput, RepoIntelligenceError> {
        Ok(PluginAnalysisOutput::default())
    }

    fn preflight_repository(
        &self,
        context: &AnalysisContext,
        repository_root: &Path,
    ) -> Result<(), RepoIntelligenceError> {
        analysis::preflight_repository(context, repository_root)
    }

    fn analyze_repository(
        &self,
        context: &AnalysisContext,
        repository_root: &Path,
    ) -> Result<RepositoryAnalysisOutput, RepoIntelligenceError> {
        analysis::analyze_repository(context, repository_root)
    }

    fn enrich_relations(
        &self,
        _context: &PluginLinkContext,
    ) -> Result<Vec<RelationRecord>, RepoIntelligenceError> {
        Ok(Vec::new())
    }
}
