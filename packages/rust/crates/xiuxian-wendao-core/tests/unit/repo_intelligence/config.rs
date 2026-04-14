use crate::repo_intelligence::{RegisteredRepository, RepositoryPluginConfig};

#[test]
fn repo_intelligence_plugin_view_excludes_search_only_plugins() {
    let repository = RegisteredRepository {
        id: "demo".to_string(),
        plugins: vec![
            RepositoryPluginConfig::Id("ast-grep".to_string()),
            RepositoryPluginConfig::Id("julia".to_string()),
            RepositoryPluginConfig::Config {
                id: "modelica".to_string(),
                options: serde_json::json!({
                    "mode": "parser-summary"
                }),
            },
        ],
        ..RegisteredRepository::default()
    };

    assert!(repository.has_repo_intelligence_plugins());
    assert_eq!(
        repository.configured_plugin_ids(),
        vec![
            "ast-grep".to_string(),
            "julia".to_string(),
            "modelica".to_string()
        ]
    );
    assert_eq!(
        repository.repo_intelligence_plugin_ids(),
        vec!["julia".to_string(), "modelica".to_string()]
    );
}

#[test]
fn repo_intelligence_plugin_view_reports_search_only_repository_as_empty() {
    let repository = RegisteredRepository {
        id: "demo".to_string(),
        plugins: vec![RepositoryPluginConfig::Id("ast-grep".to_string())],
        ..RegisteredRepository::default()
    };

    assert!(!repository.has_repo_intelligence_plugins());
    assert_eq!(
        repository.configured_plugin_ids(),
        vec!["ast-grep".to_string()]
    );
    assert!(repository.repo_intelligence_plugin_ids().is_empty());
}

#[test]
fn repo_intelligence_plugin_view_normalizes_configured_plugin_ids() {
    let repository = RegisteredRepository {
        id: "demo".to_string(),
        plugins: vec![
            RepositoryPluginConfig::Config {
                id: "modelica".to_string(),
                options: serde_json::json!({
                    "mode": "parser-summary"
                }),
            },
            RepositoryPluginConfig::Id("ast-grep".to_string()),
            RepositoryPluginConfig::Id("julia".to_string()),
            RepositoryPluginConfig::Id("ast-grep".to_string()),
            RepositoryPluginConfig::Config {
                id: "modelica".to_string(),
                options: serde_json::json!({
                    "mode": "doc-surface"
                }),
            },
        ],
        ..RegisteredRepository::default()
    };

    assert_eq!(
        repository.configured_plugin_ids(),
        vec![
            "ast-grep".to_string(),
            "julia".to_string(),
            "modelica".to_string(),
        ]
    );
    assert_eq!(
        repository.repo_intelligence_plugin_ids(),
        vec!["julia".to_string(), "modelica".to_string()]
    );
}
