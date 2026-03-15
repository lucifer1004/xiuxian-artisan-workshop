//! [AIP] DeepSeek OCR Executor Adapter for the Omni ModelBus.
//!
//! This module provides a `DeepseekExecutor` that wraps the existing
//! `DeepseekEngine` and implements the `ModelExecutor` trait for
//! hot-swap model management.

use std::sync::{Arc, OnceLock};

use async_trait::async_trait;

use crate::llm::error::{LlmError, LlmResult};
use crate::llm::vision::deepseek::{DeepseekRuntime, prewarm_deepseek_ocr};
use crate::llm::vision::{PreparedVisionImage, infer_deepseek_ocr_truth, preprocess_image};
use crate::runtime::executor::{ExecutorId, ModelExecutor, ModelInput, ModelOutput};

/// Estimated memory footprint for DeepSeek OCR models (~6GB weights).
const DEEPSEEK_MEMORY_BYTES: u64 = 6_000_000_000;

/// Executor adapter for DeepSeek OCR vision models.
///
/// This wraps the existing DeepseekEngine infrastructure and provides
/// a `ModelExecutor` interface for the Omni ModelBus hot-swap architecture.
pub struct DeepseekExecutor {
    /// Unique executor identifier.
    id: ExecutorId,
    /// The DeepSeek runtime configuration.
    runtime: DeepseekRuntime,
    /// Whether the executor has been prewarmed.
    prewarmed: OnceLock<bool>,
}

impl DeepseekExecutor {
    /// Creates a new DeepSeek executor for the given model root.
    #[must_use]
    pub fn new(model_root: &str) -> Self {
        tracing::debug!(
            event = "llm.runtime.executors.deepseek.new",
            model_root = %model_root,
            executor_id = "deepseek-ocr",
            "DeepseekExecutor: Creating new executor instance - MEMORY ALLOCATION MAY FOLLOW"
        );
        Self {
            id: ExecutorId::new("deepseek-ocr"),
            runtime: DeepseekRuntime::Configured {
                model_root: Arc::from(model_root.to_string()),
            },
            prewarmed: OnceLock::new(),
        }
    }

    /// Creates a new DeepSeek executor with explicit ID.
    #[must_use]
    pub fn with_id(id: &str, model_root: &str) -> Self {
        tracing::debug!(
            event = "llm.runtime.executors.deepseek.with_id",
            model_root = %model_root,
            executor_id = %id,
            "DeepseekExecutor: Creating new executor instance with custom ID - MEMORY ALLOCATION MAY FOLLOW"
        );
        Self {
            id: ExecutorId::new(id),
            runtime: DeepseekRuntime::Configured {
                model_root: Arc::from(model_root.to_string()),
            },
            prewarmed: OnceLock::new(),
        }
    }

    /// Preprocesses raw image bytes into PreparedVisionImage.
    fn prepare_image(image_bytes: Vec<u8>) -> LlmResult<Arc<PreparedVisionImage>> {
        let arc_bytes: Arc<[u8]> = Arc::from(image_bytes.into_boxed_slice());
        preprocess_image(arc_bytes).map(Arc::new)
    }
}

#[async_trait]
impl ModelExecutor for DeepseekExecutor {
    fn id(&self) -> &ExecutorId {
        &self.id
    }

    fn name(&self) -> &'static str {
        "deepseek-ocr"
    }

    async fn execute(&self, input: ModelInput) -> LlmResult<ModelOutput> {
        match input {
            ModelInput::Vision { prompt: _, images } => {
                if images.is_empty() {
                    return Err(LlmError::Internal {
                        message: "DeepSeek OCR requires at least one image".to_string(),
                    });
                }

                // Process the first image (current limitation)
                let prepared = Self::prepare_image(images.into_iter().next().unwrap())?;

                // Run inference using the public async API
                let result = infer_deepseek_ocr_truth(&self.runtime, &prepared, None).await?;

                match result {
                    Some(markdown) => Ok(ModelOutput::Vision(markdown)),
                    None => Ok(ModelOutput::Vision(String::new())),
                }
            }
            ModelInput::Text(_) => Err(LlmError::Internal {
                message: "DeepSeek OCR does not support text-only input".to_string(),
            }),
            ModelInput::Embedding(_) => Err(LlmError::Internal {
                message: "DeepSeek OCR does not support embedding generation".to_string(),
            }),
        }
    }

    fn prewarm(&self) -> LlmResult<()> {
        if self.prewarmed.get().is_some() {
            tracing::debug!(
                event = "llm.runtime.executors.deepseek.prewarm_skip",
                executor_id = %self.id,
                "DeepseekExecutor: Prewarm already done, skipping"
            );
            return Ok(());
        }

        tracing::debug!(
            event = "llm.runtime.executors.deepseek.prewarm_start",
            executor_id = %self.id,
            runtime = ?self.runtime,
            estimated_memory_bytes = DEEPSEEK_MEMORY_BYTES,
            "DeepseekExecutor: Starting prewarm - THIS WILL LOAD MODEL WEIGHTS INTO MEMORY"
        );

        let start = std::time::Instant::now();
        let result = prewarm_deepseek_ocr(&self.runtime);

        match &result {
            Ok(_) => {
                let _ = self.prewarmed.set(true);
                tracing::debug!(
                    event = "llm.runtime.executors.deepseek.prewarm_success",
                    executor_id = %self.id,
                    elapsed_ms = start.elapsed().as_millis(),
                    "DeepseekExecutor: Prewarm completed successfully"
                );
            }
            Err(e) => {
                tracing::error!(
                    event = "llm.runtime.executors.deepseek.prewarm_failed",
                    executor_id = %self.id,
                    error = %e,
                    elapsed_ms = start.elapsed().as_millis(),
                    "DeepseekExecutor: Prewarm failed"
                );
            }
        }

        result
    }

    fn memory_bytes(&self) -> u64 {
        DEEPSEEK_MEMORY_BYTES
    }

    fn is_ready(&self) -> bool {
        self.prewarmed.get().is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deepseek_executor_creation() {
        let executor = DeepseekExecutor::new("/test/model");
        assert_eq!(executor.id().as_str(), "deepseek-ocr");
        assert_eq!(executor.name(), "deepseek-ocr");
        assert_eq!(executor.memory_bytes(), DEEPSEEK_MEMORY_BYTES);
        assert!(!executor.is_ready());
    }

    #[test]
    fn deepseek_executor_custom_id() {
        let executor = DeepseekExecutor::with_id("custom-id", "/test/model");
        assert_eq!(executor.id().as_str(), "custom-id");
    }
}
