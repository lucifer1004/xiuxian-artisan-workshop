use crate::repo_intelligence::RepositoryPluginConfig;
use crate::repo_intelligence::{AnalysisContext, PluginAnalysisOutput, RepoSourceFile};
use crate::repo_intelligence::{
    RegisteredRepository, RepoIntelligenceError, RepoIntelligencePlugin,
};

use super::PluginRegistry;

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

#[test]
fn test_resolve_for_repository_ignores_search_only_plugins() {
    let mut registry = PluginRegistry::new();
    registry
        .register(MockPlugin("p1"))
        .unwrap_or_else(|error| panic!("plugin registration should succeed: {error}"));

    let repo = RegisteredRepository {
        id: "repo1".to_string(),
        plugins: vec![
            RepositoryPluginConfig::Id("ast-grep".to_string()),
            RepositoryPluginConfig::Id("p1".to_string()),
        ],
        ..RegisteredRepository::default()
    };

    let resolved = registry
        .resolve_for_repository(&repo)
        .unwrap_or_else(|error| panic!("plugin resolution should succeed: {error}"));
    assert_eq!(resolved.len(), 1);
    assert_eq!(resolved[0].id(), "p1");
}

#[test]
fn test_resolve_for_repository_returns_empty_for_search_only_repository() {
    let registry = PluginRegistry::new();
    let repo = RegisteredRepository {
        id: "repo1".to_string(),
        plugins: vec![RepositoryPluginConfig::Id("ast-grep".to_string())],
        ..RegisteredRepository::default()
    };

    let resolved = registry
        .resolve_for_repository(&repo)
        .unwrap_or_else(|error| {
            panic!("plugin resolution should ignore search-only plugins: {error}")
        });
    assert!(resolved.is_empty());
}
