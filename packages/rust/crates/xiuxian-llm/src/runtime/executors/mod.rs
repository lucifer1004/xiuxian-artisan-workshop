//! [AIP] Concrete `ModelExecutor` implementations for the Omni `ModelBus`.
//!
//! This module provides executor adapters for various model backends:
//! - `DeepseekExecutor`: `DeepSeek` OCR vision model adapter

mod deepseek;

pub use deepseek::DeepseekExecutor;
