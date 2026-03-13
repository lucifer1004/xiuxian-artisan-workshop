//! Shared testing utilities for xiuxian crates.
//!
//! This crate provides:
//! - **Scenario Framework**: A reusable framework for scenario-based snapshot testing
//! - **Test Utilities**: Common helpers for test setup and assertions
//!
//! # Scenario Framework
//!
//! The scenario framework allows you to define test scenarios as directories with
//! input files, expected outputs, and configuration. It uses Insta for snapshot
//! testing.
//!
//! ## Example
//!
//! ```ignore
//! use xiuxian_testing::scenario::{ScenarioFramework, ScenarioRunner, Scenario};
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
//!     framework.run_category("my_category").unwrap();
//! }
//! ```

pub mod scenario;
pub mod utils;

pub use scenario::{
    Scenario, ScenarioConfig, ScenarioFramework, ScenarioRunner, copy_dir_recursive,
    discover_scenarios, find_first_doc_name, load_expected_json, scenarios_root,
};

pub use utils::{assert_json_eq, temp_dir_with_prefix};
