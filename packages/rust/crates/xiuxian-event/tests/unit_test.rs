//! Cargo entry point for `xiuxian-event` unit tests.

xiuxian_testing::crate_test_policy_harness!();

#[path = "unit/event_bus.rs"]
mod event_bus;
#[path = "unit/lib_policy.rs"]
mod lib_policy;
