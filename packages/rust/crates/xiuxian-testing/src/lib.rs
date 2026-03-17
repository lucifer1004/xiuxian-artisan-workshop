//! Shared testing utilities for xiuxian crates.
//!
//! This crate provides:
//! - **Scenario Framework**: A reusable framework for scenario-based snapshot testing
//! - **Test Structure Validation**: Utilities to enforce test directory conventions
//! - **External Test Support**: Convention and policy validation for externalized tests via `#[path]`
//! - **Test Utilities**: Common helpers for test setup and assertions
//!
//! # Scenario Framework
//!
//! The scenario framework allows you to define test scenarios as directories with
//! input files and configuration. It uses Insta for snapshot testing.
//!
//! ## Directory Structure
//!
//! ```text
//! tests/
//! ├── scenarios/                    # Scenario definitions (you write)
//! │   └── 001_page_index_hierarchy/
//! │       ├── scenario.toml         # Metadata + assertions
//! │       └── input/                # Input files
//! ├── unit/                         # Unit tests (*.rs, snake_case)
//! ├── integration/                  # Integration tests (*.rs, snake_case)
//! └── snapshots/                    # Snapshots (insta manages)
//!     └── scenarios__*.snap
//! ```
//!
//! # External Test Module Pattern
//!
//! Keep tests out of source files using `#[path]` mounting. The shared validation path also treats
//! inline `#[cfg(test)]` modules as policy violations so crates can fail fast when tests drift back
//! into `src/`.
//! For enforcement, prefer the unified `assert_crate_test_policy` helper inside either a consumer
//! crate test or a dedicated workspace audit crate.
//!
//! ```ignore
//! // src/foo/bar.rs
//! fn business_logic() { ... }
//!
//! #[cfg(test)]
//! #[path = "../../tests/unit/foo/bar.rs"]
//! mod tests;
//! ```
//!
//! ## Example
//!
//! ```ignore
//! use xiuxian_testing::{ScenarioFramework, ScenarioRunner, Scenario};
//!
//! struct MyRunner;
//!
//! impl ScenarioRunner for MyRunner {
//!     fn category(&self) -> &str { "my_category" }
//!
//!     fn run(&self, scenario: &Scenario, temp_dir: &Path) -> Result<Value, Box<dyn Error>> {
//!         // Run your test logic here
//!         Ok(serde_json::json!({ "result": "success" }))
//!     }
//! }
//!
//! #[test]
//! fn test_my_scenarios() {
//!     let mut framework = ScenarioFramework::new();
//!     framework.register(Box::new(MyRunner));
//!     framework.run_all().unwrap();  // Runs all scenarios with registered runners
//! }
//! ```

pub mod external_test;
pub mod policy;
pub mod scenario;
pub mod utils;
pub mod validation;

pub use external_test::{
    ExternalTestMount, ExternalTestValidationIssue, calculate_test_path, generate_path_attribute,
    validate_external_test_mounts,
};

pub use policy::{
    CrateTestPolicyReport, assert_crate_test_policy, format_crate_test_policy_report,
    validate_crate_test_policy,
};

pub use scenario::{
    AssertConfig, Scenario, ScenarioConfig, ScenarioFramework, ScenarioMeta, ScenarioRunner,
    copy_dir_recursive, discover_scenarios, discover_scenarios_at, find_first_doc_name,
    load_expected_json, scenarios_root, scenarios_root_at,
};

pub use utils::{assert_json_eq, temp_dir_with_prefix};

pub use validation::{
    StructureViolation, ViolationKind, format_violation_report, validate_crate_tests,
    validate_tests_structure,
};
