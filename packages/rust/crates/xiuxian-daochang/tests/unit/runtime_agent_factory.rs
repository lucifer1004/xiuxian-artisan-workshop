//! Runtime-agent factory integration harness.

use xiuxian_daochang::{RuntimeSettings, test_support::resolve_runtime_model};

const _: fn(&RuntimeSettings) -> String = resolve_runtime_model;

#[path = "runtime_agent_factory/inference.rs"]
mod tests;
