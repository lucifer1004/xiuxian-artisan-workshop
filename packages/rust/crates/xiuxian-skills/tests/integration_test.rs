//! Cargo entry point for `xiuxian-skills` integration tests.

xiuxian_testing::crate_test_policy_harness!();

#[path = "support/json.rs"]
mod json_support;
#[path = "support/path_sanitization.rs"]
mod path_sanitization;
#[path = "support/read_fixture.rs"]
mod read_fixture_support;
#[path = "support/structure.rs"]
mod structure;
#[path = "support/write_fixture_file.rs"]
mod write_fixture_support;

#[path = "integration/full_workflow.rs"]
mod full_workflow;
#[path = "integration/schema_validation_matrix_snapshots.rs"]
mod schema_validation_matrix_snapshots;
#[path = "integration/skill_scanner_matrix_snapshots.rs"]
mod skill_scanner_matrix_snapshots;
#[path = "integration/skill_scanner_snapshots.rs"]
mod skill_scanner_snapshots;
