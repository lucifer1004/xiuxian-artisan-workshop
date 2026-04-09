//! Integration tests for `xiuxian_config` API-key policy enforcement.

use serde::Deserialize;
use std::path::{Path, PathBuf};
use tempfile::TempDir;

#[xiuxian_macros::xiuxian_config(
    namespace = "securitytest",
    internal_path = "tests/fixtures/config/securitytest.toml",
    orphan_file = ""
)]
#[derive(Debug, Deserialize, PartialEq, Eq)]
struct SecurityConfig {
    value: String,
    #[serde(default)]
    llm: Option<SecurityLlmConfig>,
}

#[derive(Debug, Deserialize, PartialEq, Eq)]
struct SecurityLlmConfig {
    providers: SecurityProviderCollection,
}

#[derive(Debug, Deserialize, PartialEq, Eq)]
struct SecurityProviderCollection {
    #[serde(default)]
    openai: Option<SecurityProviderConfig>,
}

#[derive(Debug, Deserialize, PartialEq, Eq)]
struct SecurityProviderConfig {
    api_key: String,
}

fn write_text(path: &Path, content: &str) {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .unwrap_or_else(|error| panic!("create parent {}: {error}", parent.display()));
    }
    std::fs::write(path, content)
        .unwrap_or_else(|error| panic!("write fixture {}: {error}", path.display()));
}

fn prepare_workspace() -> (TempDir, PathBuf) {
    let temp = TempDir::new().unwrap_or_else(|error| panic!("create temp fixture root: {error}"));
    let root = temp.path().to_path_buf();
    std::fs::create_dir_all(root.join(".config/xiuxian-artisan-workshop")).unwrap_or_else(
        |error| panic!("create .config/xiuxian-artisan-workshop in fixture root: {error}"),
    );
    (temp, root)
}

fn openai_api_key(config: &SecurityConfig) -> Option<&str> {
    config
        .llm
        .as_ref()
        .and_then(|llm| llm.providers.openai.as_ref())
        .map(|provider| provider.api_key.as_str())
}

#[test]
fn xiuxian_config_accepts_api_key_env_reference() {
    let (_temp, root) = prepare_workspace();
    write_text(
        root.join(".config/xiuxian-artisan-workshop/xiuxian.toml")
            .as_path(),
        r#"
[securitytest.llm.providers.openai]
api_key = "OPENAI_API_KEY"
"#,
    );

    let config =
        SecurityConfig::load_with_paths(Some(root.as_path()), Some(root.join(".config").as_path()))
            .unwrap_or_else(|error| panic!("expected env-reference api_key to pass: {error}"));
    assert_eq!(config.value, "default");
    assert_eq!(openai_api_key(&config), Some("OPENAI_API_KEY"));
}

#[test]
fn xiuxian_config_merges_plaintext_api_key_override() {
    let (_temp, root) = prepare_workspace();
    write_text(
        root.join(".config/xiuxian-artisan-workshop/xiuxian.toml")
            .as_path(),
        r#"
[securitytest.llm.providers.openai]
api_key = "sk-live-plaintext-should-fail"
"#,
    );

    let config =
        SecurityConfig::load_with_paths(Some(root.as_path()), Some(root.join(".config").as_path()))
            .unwrap_or_else(|error| {
                panic!("expected plaintext api_key override to merge: {error}")
            });
    assert_eq!(config.value, "default");
    assert_eq!(
        openai_api_key(&config),
        Some("sk-live-plaintext-should-fail")
    );
}
