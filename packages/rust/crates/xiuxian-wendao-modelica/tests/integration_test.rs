//! Cargo entry point for `xiuxian-wendao-modelica` integration tests.

xiuxian_testing::crate_test_policy_harness!();

#[path = "integration/modelica_plugin.rs"]
mod modelica_plugin;
