//! Cargo entry point for `xiuxian-sandbox` unit tests.

xiuxian_testing::crate_test_policy_harness!();

#[path = "unit/lib_policy.rs"]
mod lib_policy;
#[path = "unit/sandbox.rs"]
mod sandbox;
