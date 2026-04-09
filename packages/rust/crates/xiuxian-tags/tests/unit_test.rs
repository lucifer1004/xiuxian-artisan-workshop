//! Cargo entry point for xiuxian-tags unit tests.

xiuxian_testing::crate_test_policy_harness!();

#[path = "unit/extractor.rs"]
mod extractor;
