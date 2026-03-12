//! Scenario-based snapshot testing for xiuxian-wendao.
//!
//! Inspired by codex-rs's apply-patch test structure, this module provides
//! a standardized way to define and run snapshot tests for link graph operations.
//!
//! # Scenario Structure
//!
//! ```text
//! tests/fixtures/scenarios/001_page_index_hierarchy/
//! ├── input/
//! │   └── docs/
//! │       └── alpha.md
//! ├── expected/
//! │   └── tree.json
//! └── scenario.toml
//! ```

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use serde::Deserialize;
use serde_json::Value;

/// Scenario configuration from scenario.toml
#[derive(Debug, Clone, Deserialize)]
pub struct ScenarioConfig {
    pub scenario: ScenarioMeta,
    pub input: InputConfig,
    pub expected: ExpectedConfig,
    pub runner: RunnerConfig,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ScenarioMeta {
    pub id: String,
    pub name: String,
    pub description: String,
    pub category: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct InputConfig {
    /// Type of input: "markdown_tree", "arrow_batch", "json"
    #[serde(rename = "type")]
    pub input_type: String,
    /// Paths relative to scenario directory
    pub paths: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ExpectedConfig {
    /// Type of expected output: "json_snapshot", "text_snapshot"
    #[serde(rename = "type")]
    pub output_type: String,
    /// Expected files to compare
    pub files: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RunnerConfig {
    pub build_page_index: Option<bool>,
    pub collect_links: Option<bool>,
}

/// Get the scenarios fixture root directory
pub fn scenarios_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("scenarios")
}

/// Discover all scenario directories
pub fn discover_scenarios() -> Vec<PathBuf> {
    let root = scenarios_root();
    let mut scenarios = Vec::new();

    if let Ok(entries) = fs::read_dir(&root) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() && path.join("scenario.toml").exists() {
                scenarios.push(path);
            }
        }
    }

    // Sort by scenario ID (directory name)
    scenarios.sort();
    scenarios
}

/// Load scenario configuration
pub fn load_scenario_config(dir: &Path) -> Result<ScenarioConfig, Box<dyn std::error::Error>> {
    let config_path = dir.join("scenario.toml");
    let content = fs::read_to_string(&config_path)?;
    let config: ScenarioConfig = toml::from_str(&content)?;
    Ok(config)
}

/// Snapshot a JSON file
pub fn snapshot_json_file(path: &Path) -> Result<Value, Box<dyn std::error::Error>> {
    let content = fs::read_to_string(path)?;
    let value: Value = serde_json::from_str(&content)?;
    Ok(value)
}

/// Snapshot all expected files in a scenario
pub fn snapshot_expected(
    dir: &Path,
    config: &ExpectedConfig,
) -> Result<BTreeMap<String, Value>, Box<dyn std::error::Error>> {
    let mut snapshots = BTreeMap::new();

    for file in &config.files {
        let path = dir.join("expected").join(file);
        if path.exists() {
            let value = snapshot_json_file(&path)?;
            snapshots.insert(file.clone(), value);
        }
    }

    Ok(snapshots)
}
