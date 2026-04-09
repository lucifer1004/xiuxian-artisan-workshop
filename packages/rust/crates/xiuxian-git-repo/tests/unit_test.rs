//! Cargo entry point for xiuxian-git-repo unit tests.

xiuxian_testing::crate_test_policy_harness!();

#[path = "unit/diff.rs"]
mod diff;
#[path = "unit/layout.rs"]
mod layout;
#[path = "unit/locks.rs"]
mod locks;
#[path = "unit/materialization.rs"]
mod materialization;
#[path = "unit/retry.rs"]
mod retry;
#[path = "support/mod.rs"]
mod support;
