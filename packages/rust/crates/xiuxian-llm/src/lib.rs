//! Shared xiuxian LLM runtime primitives.

xiuxian_testing::crate_test_policy_source_harness!("../tests/unit/lib_policy.rs");

pub mod embedding;
pub mod llm;
pub mod runtime;
#[doc(hidden)]
pub mod test_support;
pub mod web;
