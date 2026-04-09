//! Cargo entry point for `xiuxian-ast` performance tests.

xiuxian_testing::crate_test_policy_harness!();

#[path = "performance/ast_benchmark.rs"]
mod ast_benchmark;
