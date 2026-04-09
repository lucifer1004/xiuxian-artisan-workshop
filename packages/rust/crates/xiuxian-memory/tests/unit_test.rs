//! Cargo entry point for `xiuxian-memory` unit tests.

xiuxian_testing::crate_test_policy_harness!();

#[path = "unit/lib_policy.rs"]
mod lib_policy;
#[path = "unit/memrl.rs"]
mod memrl;
