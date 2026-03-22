use std::collections::BTreeMap;
use std::sync::Arc;

use super::config::RegisteredRepository;
use super::errors::RepoIntelligenceError;
use super::plugin::RepoIntelligencePlugin;

/// In-memory plugin registry for Repo Intelligence analyzers.
#[derive(Default)]
pub struct PluginRegistry {
    plugins: BTreeMap<String, Arc<dyn RepoIntelligencePlugin>>,
}

impl PluginRegistry {
    /// Construct an empty plugin registry.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Register one plugin implementation.
    ///
    /// # Errors
    ///
    /// Returns [`RepoIntelligenceError`] if a plugin with the same identifier
    /// has already been registered.
    pub fn register<P>(&mut self, plugin: P) -> Result<(), RepoIntelligenceError>
    where
        P: RepoIntelligencePlugin + 'static,
    {
        let plugin_id = plugin.id().to_string();
        if self.plugins.contains_key(&plugin_id) {
            return Err(RepoIntelligenceError::DuplicatePlugin { plugin_id });
        }

        self.plugins.insert(plugin_id, Arc::new(plugin));
        Ok(())
    }

    /// Fetch a plugin by identifier.
    #[must_use]
    pub fn get(&self, plugin_id: &str) -> Option<Arc<dyn RepoIntelligencePlugin>> {
        self.plugins.get(plugin_id).map(Arc::clone)
    }

    /// Return the registered plugin identifiers in stable order.
    #[must_use]
    pub fn plugin_ids(&self) -> Vec<&str> {
        self.plugins.keys().map(String::as_str).collect()
    }

    /// Resolve all plugins configured for a repository.
    ///
    /// # Errors
    ///
    /// Returns [`RepoIntelligenceError`] when a configured plugin identifier
    /// is missing from the registry.
    pub fn resolve_for_repository(
        &self,
        repository: &RegisteredRepository,
    ) -> Result<Vec<Arc<dyn RepoIntelligencePlugin>>, RepoIntelligenceError> {
        let mut resolved = Vec::new();
        for plugin in &repository.plugins {
            let registered =
                self.get(plugin.id())
                    .ok_or_else(|| RepoIntelligenceError::MissingPlugin {
                        plugin_id: plugin.id().to_string(),
                    })?;

            if registered.supports_repository(repository) {
                resolved.push(registered);
            }
        }
        Ok(resolved)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analyzers::config::RepositoryPluginConfig;
    use crate::analyzers::plugin::{AnalysisContext, PluginAnalysisOutput, RepoSourceFile};

    struct MockPlugin(&'static str);
    impl RepoIntelligencePlugin for MockPlugin {
        fn id(&self) -> &'static str {
            self.0
        }
        fn supports_repository(&self, _repository: &RegisteredRepository) -> bool {
            true
        }
        fn analyze_file(
            &self,
            _context: &AnalysisContext,
            _file: &RepoSourceFile,
        ) -> Result<PluginAnalysisOutput, RepoIntelligenceError> {
            Ok(PluginAnalysisOutput::default())
        }
    }

    #[test]
    fn test_plugin_registration() {
        let mut registry = PluginRegistry::new();
        registry
            .register(MockPlugin("test-plugin"))
            .unwrap_or_else(|error| panic!("plugin registration should succeed: {error}"));

        assert!(registry.get("test-plugin").is_some());
        assert_eq!(registry.plugin_ids(), vec!["test-plugin"]);

        // Duplicate registration should fail
        let result = registry.register(MockPlugin("test-plugin"));
        assert!(matches!(
            result,
            Err(RepoIntelligenceError::DuplicatePlugin { .. })
        ));
    }

    #[test]
    fn test_resolve_for_repository() {
        let mut registry = PluginRegistry::new();
        registry
            .register(MockPlugin("p1"))
            .unwrap_or_else(|error| panic!("plugin registration should succeed: {error}"));
        registry
            .register(MockPlugin("p2"))
            .unwrap_or_else(|error| panic!("plugin registration should succeed: {error}"));

        let repo = RegisteredRepository {
            id: "repo1".to_string(),
            plugins: vec![
                RepositoryPluginConfig::Id("p1".to_string()),
                RepositoryPluginConfig::Id("p2".to_string()),
            ],
            ..RegisteredRepository::default()
        };

        let resolved = registry
            .resolve_for_repository(&repo)
            .unwrap_or_else(|error| panic!("plugin resolution should succeed: {error}"));
        assert_eq!(resolved.len(), 2);
        assert_eq!(resolved[0].id(), "p1");
        assert_eq!(resolved[1].id(), "p2");
    }
}
