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
//! # Directory Structure
//!
//! ```text
//! tests/
//! ├── scenarios/                    # Scenario definitions (you write)
//! │   └── 001_page_index_hierarchy/
//! │       ├── scenario.toml         # Metadata + assertions
//! │       └── input/                # Input files
//! │           └── docs/alpha.md
//! └── snapshots/                    # Snapshots (insta manages)
//!     └── scenarios__*.snap
//! ```
//!
//! # scenario.toml Format
//!
//! ```toml
//! [scenario]
//! id = "001_page_index_hierarchy"
//! name = "Page Index Hierarchy"
//! category = "page_index"
//! description = "Build hierarchical page index"
//!
//! [input]
//! type = "markdown_tree"
//!
//! [runner]
//! build_page_index = true
//!
//! [assert]  # Optional declarative assertions
//! node_count = 3
//! root_title = "Alpha"
//! ```

use std::collections::HashMap;
use std::error::Error;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use serde::Deserialize;
use serde_json::Value;

// ============================================================================
// Configuration Types
// ============================================================================

/// Scenario configuration from scenario.toml
#[derive(Debug, Clone, Deserialize)]
pub struct ScenarioConfig {
    /// Scenario metadata (id, name, description, category).
    pub scenario: ScenarioMeta,
    /// Input configuration for the scenario.
    pub input: InputConfig,
    /// Expected output configuration (optional - snapshots managed by insta).
    #[serde(default)]
    pub expected: Option<ExpectedConfig>,
    /// Runner-specific configuration options.
    #[serde(default)]
    pub runner: RunnerConfig,
    /// Declarative assertions (optional).
    #[serde(default)]
    pub assert: AssertConfig,
}

/// Scenario metadata from the `[scenario]` section.
#[derive(Debug, Clone, Deserialize)]
pub struct ScenarioMeta {
    /// Unique scenario identifier (e.g., "`001_routing_keywords_merge`").
    pub id: String,
    /// Human-readable scenario name.
    pub name: String,
    /// Detailed description of what the scenario tests.
    pub description: String,
    /// Category for runner selection (e.g., "`skill_definition`", "`page_index`").
    pub category: String,
}

/// Input configuration from the `[input]` section.
#[derive(Debug, Clone, Deserialize)]
pub struct InputConfig {
    /// Type of input: "`markdown_tree`", "`arrow_batch`", "`json`"
    #[serde(rename = "type")]
    pub input_type: String,
    /// Paths relative to scenario directory
    #[serde(default)]
    pub paths: Vec<String>,
}

/// Expected output configuration from the `[expected]` section.
///
/// This is optional when using insta snapshots, which are managed automatically
/// in `tests/snapshots/scenarios/`.
#[derive(Debug, Clone, Deserialize, Default)]
pub struct ExpectedConfig {
    /// Type of expected output: "`json_snapshot`", "`text_snapshot`"
    #[serde(rename = "type", default)]
    pub output_type: String,
    /// Expected files to compare (optional)
    #[serde(default)]
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

/// Declarative assertions from the `[assert]` section.
///
/// These are optional and provide human-readable validation rules
/// that complement insta snapshots.
#[derive(Debug, Clone, Deserialize, Default)]
pub struct AssertConfig {
    /// Expected node count (for tree structures).
    pub node_count: Option<usize>,
    /// Expected root title.
    pub root_title: Option<String>,
    /// Whether the result should have children.
    pub has_children: Option<bool>,
    /// Minimum token count.
    pub min_token_count: Option<usize>,
    /// Custom assertions as key-value pairs.
    #[serde(flatten)]
    pub custom: HashMap<String, Value>,
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
/// Each category of tests (`page_index`, `search`, `graph`, etc.) implements
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
    ///
    /// The snapshot path is relative to the crate's manifest directory:
    /// `tests/snapshots/`
    #[must_use]
    pub fn new() -> Self {
        let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        Self {
            runners: HashMap::new(),
            snapshot_path: manifest_dir.join("tests").join("snapshots"),
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
            .map(std::convert::AsRef::as_ref)
    }

    /// Run all scenarios across all categories.
    ///
    /// This discovers all scenarios and matches them to registered runners
    /// by their category. Insta handles snapshot management.
    ///
    /// # Errors
    ///
    /// Returns an error if any scenario execution fails.
    pub fn run_all(&self) -> Result<usize, Box<dyn Error>> {
        self.run_all_at(&scenarios_root())
    }

    /// Run all scenarios at a specific root directory.
    ///
    /// # Errors
    ///
    /// Returns an error if any scenario execution fails.
    pub fn run_all_at(&self, scenarios_root: &Path) -> Result<usize, Box<dyn Error>> {
        let scenarios = discover_scenarios_at(scenarios_root);
        let mut count = 0;

        for scenario_dir in scenarios {
            let scenario = Scenario::load(scenario_dir)?;

            let runner = self.find_runner(scenario.category()).ok_or_else(|| {
                io::Error::other(format!(
                    "No runner registered for scenario category '{}' (scenario: {})",
                    scenario.category(),
                    scenario.id()
                ))
            })?;

            count += 1;

            // Create temp directory and copy input
            let temp_dir = tempfile::TempDir::new()?;
            if let Some(input_path) = scenario.input_path()
                && input_path.exists()
            {
                copy_dir_recursive(&input_path, temp_dir.path())?;
            }

            // Run the scenario
            let result = runner.run(&scenario, temp_dir.path())?;

            // Use Insta for snapshot comparison
            let snapshot_path = self.snapshot_path.clone();
            let snapshot_name = format!("scenarios__{}", scenario.id());
            insta::with_settings!({
                snapshot_path => snapshot_path,
                prepend_module_to_snapshot => false,
            }, {
                insta::assert_json_snapshot!(snapshot_name, result);
            });
        }

        Ok(count)
    }

    /// Run all scenarios in a category using the registered runner.
    ///
    /// # Errors
    ///
    /// Returns an error if no runner is registered for the category or if
    /// any scenario execution fails.
    pub fn run_category(&self, category: &str) -> Result<usize, Box<dyn Error>> {
        let runner = self.find_runner(category).ok_or_else(|| {
            io::Error::other(format!("No runner registered for category: {category}"))
        })?;

        let scenarios = discover_scenarios();
        let mut count = 0;

        for scenario_dir in scenarios {
            let scenario = Scenario::load(scenario_dir)?;

            if !runner.handles_category(scenario.category()) {
                continue;
            }

            count += 1;

            // Create temp directory and copy input
            let temp_dir = tempfile::TempDir::new()?;
            if let Some(input_path) = scenario.input_path()
                && input_path.exists()
            {
                copy_dir_recursive(&input_path, temp_dir.path())?;
            }

            // Run the scenario and generate snapshot
            let result = runner.run(&scenario, temp_dir.path())?;

            // Use Insta for snapshot comparison
            let snapshot_path = self.snapshot_path.clone();
            let snapshot_name = format!("scenarios__{}", scenario.id());
            insta::with_settings!({
                snapshot_path => snapshot_path,
                prepend_module_to_snapshot => false,
            }, {
                insta::assert_json_snapshot!(snapshot_name, result);
            });
        }

        Ok(count)
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
/// This returns the crate-local `tests/scenarios` directory,
/// allowing each crate to define its own scenario tests.
#[must_use]
pub fn scenarios_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("scenarios")
}

/// Get a custom scenarios root directory.
#[must_use]
pub fn scenarios_root_at(base: &Path) -> PathBuf {
    base.join("tests").join("scenarios")
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
        } else if path.extension().is_some_and(|ext| ext == "md") {
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
    use std::fs;

    fn write_scenario_fixture(root: &Path, name: &str, category: &str) {
        let scenario_dir = root.join(name);
        fs::create_dir_all(&scenario_dir).expect("scenario dir should be created");
        fs::write(
            scenario_dir.join("scenario.toml"),
            format!(
                r#"[scenario]
id = "{name}"
name = "Fixture Scenario"
description = "Fixture"
category = "{category}"

[input]
type = "json"
"#
            ),
        )
        .expect("scenario.toml should be written");
    }

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

    #[test]
    fn test_scenarios_root_returns_crate_local() {
        let root = scenarios_root();
        // Should return crate-local path, not workspace root
        assert!(
            root.ends_with("tests/scenarios"),
            "scenarios_root should end with tests/scenarios: {root:?}"
        );
    }

    #[test]
    fn test_discover_scenarios_returns_local() {
        let scenarios = discover_scenarios();
        // May be empty if no local scenarios exist - that's fine
        // The key is that it looks in the crate-local directory
        for scenario in &scenarios {
            assert!(
                scenario.to_string_lossy().contains("tests/scenarios"),
                "Scenario should be in crate-local path: {scenario:?}"
            );
        }
    }

    #[test]
    fn run_all_at_fails_when_scenario_has_no_registered_runner() {
        let temp = tempfile::tempdir().expect("tempdir should be created");
        write_scenario_fixture(temp.path(), "001_missing_runner", "missing_runner");

        let framework = ScenarioFramework::new();
        let error = framework
            .run_all_at(temp.path())
            .expect_err("missing runner should fail closed");

        assert!(
            error
                .to_string()
                .contains("No runner registered for scenario category 'missing_runner'"),
            "unexpected error: {error}"
        );
    }
}
