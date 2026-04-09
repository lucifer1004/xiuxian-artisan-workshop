//! Cargo entry point for `xiuxian-ast` scenario tests.

xiuxian_testing::crate_test_policy_harness!();

#[cfg(feature = "julia")]
#[path = "integration/julia_scenarios.rs"]
mod julia_scenarios;
#[cfg(feature = "modelica")]
#[path = "integration/modelica_scenarios.rs"]
mod modelica_scenarios;
