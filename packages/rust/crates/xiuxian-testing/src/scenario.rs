//! Reusable scenario testing framework with Insta snapshot support.
//!
//! This module provides a standardized way to define and run scenario-based
//! snapshot tests.
//!
//! # Architecture
//!
//! - `Scenario`: Represents a test scenario with its configuration
//! - `ScenarioRunner`: Trait for category-specific test execution
//! - `ScenarioFramework`: Registry and executor for runners
//!
//! # Scenario Structure
//!
//! ```text
//! tests/fixtures/scenarios/001_my_scenario/
//! ├── input/
//! │   └── data.md
//! ├── expected/
//! │   └── result.json
//! └── scenario.toml
//! ```

use std::collections::HashMap;
use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};

use serde::Deserialize;
use serde_json::Value;

// ============================================================================
// Configuration Types
// ============================================================================

/// Scenario configuration from scenario.toml
#[derive(Debug, Clone, Deserialize)]
pub struct ScenarioConfig {
    pub scenario: ScenarioMeta,
    pub input: InputConfig,
    pub expected: ExpectedConfig,
    #[serde(default)]
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
    #[serde(default)]
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

/// Runner-specific configuration options.
#[derive(Debug, Clone, Deserialize, Default)]
pub struct RunnerConfig {
    /// Whether to build page indices during scenario execution.
    pub build_page_index: Option<bool>,
    /// Whether to collect links during scenario execution.
    pub collect_links: Option<bool>,
}

// ============================================================================
// Scenario
// ============================================================================

/// Represents a single test scenario.
pub struct Scenario {
    /// The directory containing the scenario
    pub dir: PathBuf,
    /// The loaded configuration
    pub config: ScenarioConfig,
}

impl Scenario {
    /// Load a scenario from a directory.
    ///
    /// # Errors
    ///
    /// Returns an error if the scenario.toml file cannot be read or parsed.
    pub fn load(dir: PathBuf) -> Result<Self, Box<dyn Error>> {
        let config_path = dir.join("scenario.toml");
        let content = fs::read_to_string(&config_path)?;
        let config: ScenarioConfig = toml::from_str(&content)?;
        Ok(Self { dir, config })
    }

    /// Get the scenario ID.
    #[must_use]
    pub fn id(&self) -> &str {
        &self.config.scenario.id
    }

    /// Get the scenario name.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.config.scenario.name
    }

    /// Get the scenario category.
    #[must_use]
    pub fn category(&self) -> &str {
        &self.config.scenario.category
    }

    /// Get the input path (first path in config).
    #[must_use]
    pub fn input_path(&self) -> Option<PathBuf> {
        self.config.input.paths.first().map(|p| self.dir.join(p))
    }

    /// Check if this scenario has input files.
    #[must_use]
    pub fn has_input(&self) -> bool {
        !self.config.input.paths.is_empty()
    }
}

// ============================================================================
// ScenarioRunner Trait
// ============================================================================

/// Trait for category-specific scenario execution.
///
/// Each category of tests (page_index, search, graph, etc.) implements
/// this trait to define how to run scenarios and generate snapshots.
pub trait ScenarioRunner: Send + Sync {
    /// Get the category name this runner handles.
    fn category(&self) -> &str;

    /// Get additional categories this runner handles (for multi-category runners).
    fn additional_categories(&self) -> Vec<&str> {
        vec![]
    }

    /// Check if this runner handles the given category.
    fn handles_category(&self, category: &str) -> bool {
        self.category() == category || self.additional_categories().contains(&category)
    }

    /// Run the scenario and return the result as JSON for snapshot comparison.
    ///
    /// # Errors
    ///
    /// Returns an error if the scenario execution fails.
    fn run(&self, scenario: &Scenario, temp_dir: &Path) -> Result<Value, Box<dyn Error>>;
}

// ============================================================================
// ScenarioFramework
// ============================================================================

/// Framework for registering and running scenario tests.
pub struct ScenarioFramework {
    runners: HashMap<String, Box<dyn ScenarioRunner>>,
    snapshot_path: PathBuf,
}

impl ScenarioFramework {
    /// Create a new empty framework with default snapshot path.
    #[must_use]
    pub fn new() -> Self {
        Self {
            runners: HashMap::new(),
            snapshot_path: PathBuf::from("../snapshots/scenarios"),
        }
    }

    /// Create a new framework with a custom snapshot path.
    #[must_use]
    pub fn with_snapshot_path(snapshot_path: impl Into<PathBuf>) -> Self {
        Self {
            runners: HashMap::new(),
            snapshot_path: snapshot_path.into(),
        }
    }

    /// Register a scenario runner.
    pub fn register(&mut self, runner: Box<dyn ScenarioRunner>) {
        let category = runner.category().to_string();
        self.runners.insert(category, runner);
    }

    /// Find the runner for a given category.
    #[must_use]
    pub fn find_runner(&self, category: &str) -> Option<&dyn ScenarioRunner> {
        self.runners
            .values()
            .find(|r| r.handles_category(category))
            .map(|b| b.as_ref())
    }

    /// Run all scenarios in a category using the registered runner.
    ///
    /// # Errors
    ///
    /// Returns an error if no runner is registered for the category or if
    /// any scenario execution fails.
    pub fn run_category(&self, category: &str) -> Result<(), Box<dyn Error>> {
        let runner = self
            .find_runner(category)
            .ok_or_else(|| format!("No runner registered for category: {}", category))?;

        let scenarios = discover_scenarios();
        let mut ran_any = false;

        for scenario_dir in scenarios {
            let scenario = Scenario::load(scenario_dir)?;

            if !runner.handles_category(scenario.category()) {
                continue;
            }

            ran_any = true;
            println!("Running scenario: {} ({})", scenario.name(), scenario.id());

            // Create temp directory and copy input
            let temp_dir = tempfile::TempDir::new()?;
            if let Some(input_path) = scenario.input_path() {
                if input_path.exists() {
                    copy_dir_recursive(&input_path, temp_dir.path())?;
                }
            }

            // Run the scenario and generate snapshot
            let result = runner.run(&scenario, temp_dir.path())?;

            // Use Insta for snapshot comparison
            let snapshot_path = self.snapshot_path.clone();
            insta::with_settings!({
                snapshot_path => snapshot_path,
                prepend_module_to_snapshot => false,
            }, {
                insta::assert_json_snapshot!(format!("{}-result", scenario.id()), result);
            });
        }

        if !ran_any {
            println!("No scenarios found for category: {}", category);
        }

        Ok(())
    }
}

impl Default for ScenarioFramework {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Utility Functions
// ============================================================================

/// Get the scenarios fixture root directory for the current crate.
///
/// This function uses `CARGO_MANIFEST_DIR` to locate the fixtures directory.
#[must_use]
pub fn scenarios_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("scenarios")
}

/// Get a custom scenarios root directory.
#[must_use]
pub fn scenarios_root_at(base: &Path) -> PathBuf {
    base.join("tests").join("fixtures").join("scenarios")
}

/// Discover all scenario directories in the default location.
#[must_use]
pub fn discover_scenarios() -> Vec<PathBuf> {
    discover_scenarios_at(&scenarios_root())
}

/// Discover all scenario directories at a specific root.
#[must_use]
pub fn discover_scenarios_at(root: &Path) -> Vec<PathBuf> {
    let mut scenarios = Vec::new();

    if let Ok(entries) = fs::read_dir(root) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() && path.join("scenario.toml").exists() {
                scenarios.push(path);
            }
        }
    }

    scenarios.sort();
    scenarios
}

/// Copy a directory recursively.
///
/// # Errors
///
/// Returns an error if any file operation fails.
pub fn copy_dir_recursive(src: &Path, dst: &Path) -> Result<(), Box<dyn Error>> {
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let path = entry.path();
        let dest_path = dst.join(entry.file_name());

        if path.is_dir() {
            fs::create_dir_all(&dest_path)?;
            copy_dir_recursive(&path, &dest_path)?;
        } else {
            if let Some(parent) = dest_path.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::copy(&path, &dest_path)?;
        }
    }
    Ok(())
}

/// Find the first markdown file name in a directory tree.
///
/// # Errors
///
/// Returns an error if no markdown file is found.
pub fn find_first_doc_name(dir: &Path) -> Result<String, Box<dyn Error>> {
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();

        if path.is_dir() {
            if let Ok(name) = find_first_doc_name(&path) {
                return Ok(name);
            }
        } else if path.extension().map_or(false, |ext| ext == "md") {
            let stem = path
                .file_stem()
                .ok_or("missing file stem")?
                .to_string_lossy()
                .to_string();
            return Ok(stem);
        }
    }
    Err("no markdown file found".into())
}

/// Load expected JSON from a scenario's expected directory.
///
/// # Errors
///
/// Returns an error if the file cannot be read or parsed as JSON.
pub fn load_expected_json(scenario_dir: &Path, file: &str) -> Result<Value, Box<dyn Error>> {
    let path = scenario_dir.join("expected").join(file);
    let content = fs::read_to_string(&path)?;
    let value: Value = serde_json::from_str(&content)?;
    Ok(value)
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_framework_new() {
        let framework = ScenarioFramework::new();
        assert!(framework.find_runner("nonexistent").is_none());
    }

    #[test]
    fn test_scenario_config_default() {
        let config = RunnerConfig::default();
        assert!(config.build_page_index.is_none());
        assert!(config.collect_links.is_none());
    }
}
