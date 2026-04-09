//! Cargo entry point for `xiuxian-logging` unit tests.

xiuxian_testing::crate_test_policy_harness!();

#[path = "unit/lib_policy.rs"]
mod lib_policy;
#[path = "unit/logging_args.rs"]
mod logging_args;
