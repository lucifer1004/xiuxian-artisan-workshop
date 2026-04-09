//! Cargo entry point for xiuxian-macros unit tests.

xiuxian_testing::crate_test_policy_harness!();

#[path = "unit/macros.rs"]
mod macros;
#[path = "unit/xiuxian_config_api_key_policy.rs"]
mod xiuxian_config_api_key_policy;
