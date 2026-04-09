//! Cargo entry point for xiuxian-types unit tests.

xiuxian_testing::crate_test_policy_harness!();

#[path = "unit/scenarios.rs"]
mod scenarios;
#[path = "unit/skill_definition.rs"]
mod skill_definition;
