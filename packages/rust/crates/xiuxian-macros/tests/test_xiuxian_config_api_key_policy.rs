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
}

#[test]
fn xiuxian_config_rejects_plaintext_api_key() {
    let (_temp, root) = prepare_workspace();
    write_text(
        root.join(".config/xiuxian-artisan-workshop/xiuxian.toml")
            .as_path(),
        r#"
[securitytest.llm.providers.openai]
api_key = "sk-live-plaintext-should-fail"
"#,
    );

    let error =
        SecurityConfig::load_with_paths(Some(root.as_path()), Some(root.join(".config").as_path()))
            .err()
            .unwrap_or_else(|| panic!("expected plaintext api_key to fail security gate"));
    assert!(
        error.contains("Plaintext `api_key` values are forbidden"),
        "expected plaintext api_key error message, got: {error}"
    );
    assert!(
        error.contains("llm.providers.openai.api_key"),
        "expected failing path in error message, got: {error}"
    );
}
