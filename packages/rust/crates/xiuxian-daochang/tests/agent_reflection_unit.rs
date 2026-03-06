//! Top-level integration harness for `agent::reflection`.

mod contracts {
    pub(crate) use xiuxian_daochang::{
        OmegaFallbackPolicy, OmegaRiskLevel, OmegaRoute, OmegaToolTrustClass,
    };
}

use xiuxian_daochang::test_support::{
    ReflectiveRuntime, ReflectiveRuntimeStage, build_turn_reflection, derive_policy_hint,
};

#[path = "agent/reflection/tests.rs"]
mod tests;
