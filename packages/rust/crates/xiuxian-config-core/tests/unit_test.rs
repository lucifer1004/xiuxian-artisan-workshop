//! Cargo entry point for `xiuxian-config-core` unit tests.

xiuxian_testing::crate_test_policy_harness!();

#[path = "unit/cache.rs"]
mod cache;
#[path = "unit/imports.rs"]
mod imports;
#[path = "unit/lib_policy.rs"]
mod lib_policy;
#[path = "unit/paths.rs"]
mod paths;
#[path = "unit/resolve.rs"]
mod resolve;
