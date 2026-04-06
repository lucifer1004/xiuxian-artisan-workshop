//! Unified gate entry point for `xiuxian-git-repo` modular tests.

#![allow(clippy::expect_used)]

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
