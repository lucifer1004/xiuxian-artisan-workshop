//! Cargo entry point for xiuxian-zhenfa unit tests.

xiuxian_testing::crate_test_policy_harness!();

#[path = "unit/notification.rs"]
mod notification;
#[path = "unit/signal_registry.rs"]
mod signal_registry;
