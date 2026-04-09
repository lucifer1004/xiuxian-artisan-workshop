//! Cargo entry point for xiuxian-tokenizer unit tests.

xiuxian_testing::crate_test_policy_harness!();

#[path = "unit/tokenizer.rs"]
mod tokenizer;
#[path = "unit/tokenizer_benchmark.rs"]
mod tokenizer_benchmark;
