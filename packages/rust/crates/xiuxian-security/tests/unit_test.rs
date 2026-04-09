//! Cargo entry point for `xiuxian-security` unit tests.

xiuxian_testing::crate_test_policy_harness!();

#[path = "unit/lib_policy.rs"]
mod lib_policy;
#[path = "unit/sandbox.rs"]
mod sandbox;
#[path = "unit/security.rs"]
mod security;
