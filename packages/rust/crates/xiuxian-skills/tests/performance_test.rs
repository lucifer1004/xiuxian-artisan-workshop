//! Cargo entry point for `xiuxian-skills` performance tests.

xiuxian_testing::crate_test_policy_harness!();

#[path = "performance/benchmark.rs"]
mod benchmark;
