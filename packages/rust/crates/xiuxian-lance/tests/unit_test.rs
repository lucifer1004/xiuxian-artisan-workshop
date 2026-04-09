//! Cargo entry point for `xiuxian-lance` unit tests.

xiuxian_testing::crate_test_policy_harness!();

#[path = "unit/lance.rs"]
mod lance;
#[path = "unit/lib_policy.rs"]
mod lib_policy;
