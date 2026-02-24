//! Built-in node execution mechanisms for the Qianji Box.

/// Context annotation mechanism.
pub mod annotation;
/// Adversarial calibration mechanism (Synapse-Audit).
pub mod calibration;
/// Wendao knowledge retrieval mechanism.
pub mod knowledge;
/// Mock mechanism for testing.
pub mod mock;
/// Probabilistic MDP routing mechanism.
pub mod router;

#[cfg(feature = "llm")]
/// LLM analysis mechanism.
pub mod llm;

pub use mock::MockMechanism;
