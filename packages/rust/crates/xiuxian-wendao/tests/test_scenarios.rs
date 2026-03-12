//! Scenario-based snapshot tests for xiuxian-wendao.
//!
//! Inspired by codex-rs's apply-patch test structure:
//! - Each scenario is a numbered directory with input/expected subdirs
//! - Tests run automatically by discovering scenarios
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

#[path = "support/fixture_json_assertions.rs"]
mod fixture_json_assertions;
#[path = "support/fixture_read.rs"]
mod fixture_read;
#[path = "support/link_graph_fixture_tree.rs"]
mod link_graph_fixture_tree;

use std::fs;
use std::path::{Path, PathBuf};

use fixture_json_assertions::assert_json_fixture_eq;
use serde::Deserialize;
use serde_json::{Value, json};
use xiuxian_wendao::{LinkGraphIndex, link_graph::PageIndexNode};

// ============================================================================
// Scenario Configuration Types
// ============================================================================

#[derive(Debug, Clone, Deserialize)]
struct ScenarioConfig {
    scenario: ScenarioMeta,
    input: InputConfig,
    expected: ExpectedConfig,
    runner: RunnerConfig,
}

#[derive(Debug, Clone, Deserialize)]
struct ScenarioMeta {
    id: String,
    name: String,
    description: String,
    category: String,
}

#[derive(Debug, Clone, Deserialize)]
struct InputConfig {
    #[serde(rename = "type")]
    input_type: String,
    paths: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
struct ExpectedConfig {
    #[serde(rename = "type")]
    output_type: String,
    files: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
struct RunnerConfig {
    build_page_index: Option<bool>,
    collect_links: Option<bool>,
}

// ============================================================================
// Scenario Discovery and Loading
// ============================================================================

fn scenarios_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("scenarios")
}

fn discover_scenarios() -> Vec<PathBuf> {
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

    scenarios.sort();
    scenarios
}

fn load_scenario_config(dir: &Path) -> Result<ScenarioConfig, Box<dyn std::error::Error>> {
    let config_path = dir.join("scenario.toml");
    let content = fs::read_to_string(&config_path)?;
    let config: ScenarioConfig = toml::from_str(&content)?;
    Ok(config)
}

// ============================================================================
// Utility Functions
// ============================================================================

fn copy_dir_recursive(src: &Path, dst: &Path) -> Result<(), Box<dyn std::error::Error>> {
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

fn find_first_doc_name(dir: &Path) -> Result<String, Box<dyn std::error::Error>> {
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

fn load_expected_json(dir: &Path, file: &str) -> Result<Value, Box<dyn std::error::Error>> {
    let path = dir.join("expected").join(file);
    let content = fs::read_to_string(&path)?;
    let value: Value = serde_json::from_str(&content)?;
    Ok(value)
}

// ============================================================================
// Page Index Tree Snapshot (from existing support)
// ============================================================================

fn page_index_tree_snapshot(nodes: &[PageIndexNode]) -> Value {
    json!({
        "nodes": nodes.iter().map(snapshot_page_index_node).collect::<Vec<_>>(),
    })
}

fn snapshot_page_index_node(node: &PageIndexNode) -> Value {
    json!({
        "node_id": node.node_id,
        "title": node.title,
        "level": node.level,
        "text": node.text.as_ref(),
        "summary": node.summary,
        "line_range": [node.metadata.line_range.0, node.metadata.line_range.1],
        "token_count": node.metadata.token_count,
        "is_thinned": node.metadata.is_thinned,
        "children": node.children.iter().map(snapshot_page_index_node).collect::<Vec<_>>(),
    })
}

// ============================================================================
// Test: Page Index Scenarios
// ============================================================================

#[test]
fn test_page_index_scenarios() -> Result<(), Box<dyn std::error::Error>> {
    let scenarios = discover_scenarios();

    for scenario_dir in &scenarios {
        let config = load_scenario_config(scenario_dir)?;

        if config.scenario.category != "page_index" {
            continue;
        }

        println!(
            "Running scenario: {} ({})",
            config.scenario.name, config.scenario.id
        );

        // Build fixture from input directory
        let input_path = scenario_dir.join(&config.input.paths[0]);
        let temp_dir = tempfile::TempDir::new()?;
        copy_dir_recursive(&input_path, temp_dir.path())?;

        // Build the index
        let index = LinkGraphIndex::build(temp_dir.path())?;

        // Get the expected doc name from input
        let doc_name = find_first_doc_name(temp_dir.path())?;
        let roots = index
            .page_index(&doc_name)
            .ok_or_else(|| format!("missing page index for {}", doc_name))?;

        // Generate actual snapshot
        let actual = page_index_tree_snapshot(roots);

        // Compare with expected
        for file in &config.expected.files {
            if file == "tree.json" {
                let expected = load_expected_json(scenario_dir, file)?;
                assert_eq!(
                    actual, expected,
                    "Scenario {} tree.json mismatch",
                    config.scenario.id
                );
            }
        }
    }

    Ok(())
}

// ============================================================================
// Test: Quantum Fusion Scenarios
// ============================================================================

#[test]
fn test_quantum_fusion_scenarios() -> Result<(), Box<dyn std::error::Error>> {
    let scenarios = discover_scenarios();

    for scenario_dir in &scenarios {
        let config = load_scenario_config(scenario_dir)?;

        if config.scenario.category != "quantum_fusion" && config.scenario.category != "hybrid" {
            continue;
        }

        println!(
            "Running scenario: {} ({})",
            config.scenario.name, config.scenario.id
        );

        // For quantum fusion, we need the hybrid fixture structure
        // These tests are covered by test_link_graph_quantum_fusion.rs
        // Just verify the scenario files exist and are valid
        for file in &config.expected.files {
            let expected = load_expected_json(scenario_dir, file)?;
            println!(
                "  - {} has {} keys",
                file,
                expected.as_object().map_or(0, |o| o.len())
            );
        }
    }

    Ok(())
}

// ============================================================================
// Test: Search Core Scenarios
// ============================================================================

#[test]
fn test_search_core_scenarios() -> Result<(), Box<dyn std::error::Error>> {
    let scenarios = discover_scenarios();
    let search_categories = [
        "search_core",
        "search_filters",
        "search_match_strategies",
        "tree_scope_filters",
    ];

    for scenario_dir in &scenarios {
        let config = load_scenario_config(scenario_dir)?;

        if !search_categories.contains(&config.scenario.category.as_str()) {
            continue;
        }

        println!(
            "Running scenario: {} ({})",
            config.scenario.name, config.scenario.id
        );

        // Build fixture from input directory
        if config.input.paths.is_empty() {
            println!("  - No input, skipping build");
            continue;
        }

        let input_path = scenario_dir.join(&config.input.paths[0]);
        if !input_path.exists() {
            println!("  - Input path does not exist, skipping");
            continue;
        }

        let temp_dir = tempfile::TempDir::new()?;
        copy_dir_recursive(&input_path, temp_dir.path())?;

        // Build the index
        let index = LinkGraphIndex::build(temp_dir.path())?;

        // Verify expected files exist and are valid JSON
        for file in &config.expected.files {
            let expected = load_expected_json(scenario_dir, file)?;
            println!(
                "  - {} has {} entries",
                file,
                expected
                    .get("hits")
                    .and_then(|h| h.as_array())
                    .map_or(0, |a| a.len())
            );
        }
    }

    Ok(())
}

// ============================================================================
// Test: Build Scope Scenarios
// ============================================================================

#[test]
fn test_build_scope_scenarios() -> Result<(), Box<dyn std::error::Error>> {
    let scenarios = discover_scenarios();

    for scenario_dir in &scenarios {
        let config = load_scenario_config(scenario_dir)?;

        if config.scenario.category != "build_scope" {
            continue;
        }

        println!(
            "Running scenario: {} ({})",
            config.scenario.name, config.scenario.id
        );

        // Build fixture from input directory
        let input_path = scenario_dir.join(&config.input.paths[0]);
        let temp_dir = tempfile::TempDir::new()?;
        copy_dir_recursive(&input_path, temp_dir.path())?;

        // Build the index
        let index = LinkGraphIndex::build(temp_dir.path())?;

        // Verify expected files exist and are valid JSON
        for file in &config.expected.files {
            let expected = load_expected_json(scenario_dir, file)?;
            println!("  - {} validated", file);
        }
    }

    Ok(())
}

// ============================================================================
// Test: Graph Navigation Scenarios
// ============================================================================

#[test]
fn test_graph_navigation_scenarios() -> Result<(), Box<dyn std::error::Error>> {
    let scenarios = discover_scenarios();
    let nav_categories = ["graph_navigation", "mixed_topology"];

    for scenario_dir in &scenarios {
        let config = load_scenario_config(scenario_dir)?;

        if !nav_categories.contains(&config.scenario.category.as_str()) {
            continue;
        }

        println!(
            "Running scenario: {} ({})",
            config.scenario.name, config.scenario.id
        );

        // Build fixture from input directory
        let input_path = scenario_dir.join(&config.input.paths[0]);
        let temp_dir = tempfile::TempDir::new()?;
        copy_dir_recursive(&input_path, temp_dir.path())?;

        // Build the index
        let index = LinkGraphIndex::build(temp_dir.path())?;

        // Verify expected files exist and are valid JSON
        for file in &config.expected.files {
            let expected = load_expected_json(scenario_dir, file)?;
            println!("  - {} validated", file);
        }
    }

    Ok(())
}

// ============================================================================
// Test: Cache Build Scenarios
// ============================================================================

#[test]
fn test_cache_build_scenarios() -> Result<(), Box<dyn std::error::Error>> {
    let scenarios = discover_scenarios();

    for scenario_dir in &scenarios {
        let config = load_scenario_config(scenario_dir)?;

        if config.scenario.category != "cache_build" {
            continue;
        }

        println!(
            "Running scenario: {} ({})",
            config.scenario.name, config.scenario.id
        );

        // Build fixture from input directory
        let input_path = scenario_dir.join(&config.input.paths[0]);
        let temp_dir = tempfile::TempDir::new()?;
        copy_dir_recursive(&input_path, temp_dir.path())?;

        // Build the index
        let index = LinkGraphIndex::build(temp_dir.path())?;

        // Verify expected files exist and are valid JSON
        for file in &config.expected.files {
            let expected = load_expected_json(scenario_dir, file)?;
            println!("  - {} validated", file);
        }
    }

    Ok(())
}

// ============================================================================
// Test: Markdown Attachments Scenarios
// ============================================================================

#[test]
fn test_markdown_attachments_scenarios() -> Result<(), Box<dyn std::error::Error>> {
    let scenarios = discover_scenarios();

    for scenario_dir in &scenarios {
        let config = load_scenario_config(scenario_dir)?;

        if config.scenario.category != "markdown_attachments" {
            continue;
        }

        println!(
            "Running scenario: {} ({})",
            config.scenario.name, config.scenario.id
        );

        // Build fixture from input directory
        let input_path = scenario_dir.join(&config.input.paths[0]);
        let temp_dir = tempfile::TempDir::new()?;
        copy_dir_recursive(&input_path, temp_dir.path())?;

        // Build the index
        let index = LinkGraphIndex::build(temp_dir.path())?;

        // Verify expected files exist and are valid JSON
        for file in &config.expected.files {
            let expected = load_expected_json(scenario_dir, file)?;
            println!("  - {} validated", file);
        }
    }

    Ok(())
}

// ============================================================================
// Test: Refresh Scenarios
// ============================================================================

#[test]
fn test_refresh_scenarios() -> Result<(), Box<dyn std::error::Error>> {
    let scenarios = discover_scenarios();

    for scenario_dir in &scenarios {
        let config = load_scenario_config(scenario_dir)?;

        if config.scenario.category != "refresh" {
            continue;
        }

        println!(
            "Running scenario: {} ({})",
            config.scenario.name, config.scenario.id
        );

        // Build fixture from input directory
        let input_path = scenario_dir.join(&config.input.paths[0]);
        let temp_dir = tempfile::TempDir::new()?;
        copy_dir_recursive(&input_path, temp_dir.path())?;

        // Build the index
        let index = LinkGraphIndex::build(temp_dir.path())?;

        // Verify expected files exist and are valid JSON
        for file in &config.expected.files {
            let expected = load_expected_json(scenario_dir, file)?;
            println!("  - {} validated", file);
        }
    }

    Ok(())
}

// ============================================================================
// Test: Seed and Priors Scenarios
// ============================================================================

#[test]
fn test_seed_and_priors_scenarios() -> Result<(), Box<dyn std::error::Error>> {
    let scenarios = discover_scenarios();

    for scenario_dir in &scenarios {
        let config = load_scenario_config(scenario_dir)?;

        if config.scenario.category != "seed_and_priors" {
            continue;
        }

        println!(
            "Running scenario: {} ({})",
            config.scenario.name, config.scenario.id
        );

        // Build fixture from input directory
        let input_path = scenario_dir.join(&config.input.paths[0]);
        let temp_dir = tempfile::TempDir::new()?;
        copy_dir_recursive(&input_path, temp_dir.path())?;

        // Build the index
        let index = LinkGraphIndex::build(temp_dir.path())?;

        // Verify expected files exist and are valid JSON
        for file in &config.expected.files {
            let expected = load_expected_json(scenario_dir, file)?;
            println!("  - {} validated", file);
        }
    }

    Ok(())
}

// ============================================================================
// Test: Semantic Policy Scenarios
// ============================================================================

#[test]
fn test_semantic_policy_scenarios() -> Result<(), Box<dyn std::error::Error>> {
    let scenarios = discover_scenarios();

    for scenario_dir in &scenarios {
        let config = load_scenario_config(scenario_dir)?;

        if config.scenario.category != "semantic_policy" {
            continue;
        }

        println!(
            "Running scenario: {} ({})",
            config.scenario.name, config.scenario.id
        );

        // Semantic policy tests may not have input files
        // Just verify expected files exist and are valid JSON
        for file in &config.expected.files {
            let expected = load_expected_json(scenario_dir, file)?;
            println!("  - {} validated", file);
        }
    }

    Ok(())
}

// ============================================================================
// Test: PPR Scenarios
// ============================================================================

#[test]
fn test_ppr_scenarios() -> Result<(), Box<dyn std::error::Error>> {
    let scenarios = discover_scenarios();
    let ppr_categories = ["ppr_precision", "ppr_weighting"];

    for scenario_dir in &scenarios {
        let config = load_scenario_config(scenario_dir)?;

        if !ppr_categories.contains(&config.scenario.category.as_str()) {
            continue;
        }

        println!(
            "Running scenario: {} ({})",
            config.scenario.name, config.scenario.id
        );

        // PPR tests may not have input files
        // Just verify expected files exist and are valid JSON
        for file in &config.expected.files {
            let expected = load_expected_json(scenario_dir, file)?;
            println!("  - {} validated", file);
        }
    }

    Ok(())
}

// ============================================================================
// Test: Agentic Expansion Scenarios
// ============================================================================

#[test]
fn test_agentic_expansion_scenarios() -> Result<(), Box<dyn std::error::Error>> {
    let scenarios = discover_scenarios();

    for scenario_dir in &scenarios {
        let config = load_scenario_config(scenario_dir)?;

        if config.scenario.category != "agentic_expansion" {
            continue;
        }

        println!(
            "Running scenario: {} ({})",
            config.scenario.name, config.scenario.id
        );

        // Build fixture from input directory
        let input_path = scenario_dir.join(&config.input.paths[0]);
        let temp_dir = tempfile::TempDir::new()?;
        copy_dir_recursive(&input_path, temp_dir.path())?;

        // Build the index
        let index = LinkGraphIndex::build(temp_dir.path())?;

        // Verify expected files exist and are valid JSON
        for file in &config.expected.files {
            let expected = load_expected_json(scenario_dir, file)?;
            println!("  - {} validated", file);
        }
    }

    Ok(())
}
