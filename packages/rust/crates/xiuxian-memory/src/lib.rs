//! xiuxian-memory: `MemRL` self-evolving memory system.

xiuxian_testing::crate_test_policy_source_harness!("../tests/unit/lib_policy.rs");

pub mod core;

pub use core::learner::MemRLCortex;
pub use core::types::{MemoryAction, MemoryState};
