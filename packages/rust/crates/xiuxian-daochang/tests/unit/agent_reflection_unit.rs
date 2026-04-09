//! Top-level integration harness for `agent::reflection`.

use xiuxian_daochang::test_support::{
    ReflectiveRuntime, ReflectiveRuntimeStage, build_turn_reflection, derive_policy_hint,
};

#[path = "agent/reflection/tests.rs"]
mod tests;
