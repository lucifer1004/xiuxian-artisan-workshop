//! Cargo entry point for `xiuxian-window` unit tests.

xiuxian_testing::crate_test_policy_harness!();

#[path = "unit/lib_policy.rs"]
mod lib_policy;
#[path = "unit/window.rs"]
mod window;
