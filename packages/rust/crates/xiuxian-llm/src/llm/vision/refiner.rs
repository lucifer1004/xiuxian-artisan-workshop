use std::cmp::Ordering;
use std::sync::Arc;

use super::anchor::{TextAnchor, VisualAnchor};
use super::cot::{VisualCotInput, VisualCotMode, build_visual_cot_prompt};
use super::deepseek::{DeepseekRuntime, get_deepseek_runtime, infer_deepseek_ocr_truth};
use super::preprocess::{
    DEFAULT_VISION_MAX_DIMENSION, PreparedVisionImage, preprocess_image_with_max_dimension,
};
use crate::llm::error::LlmResult;

/// Result container returned by [`VisualRefiner::refine`].
#[derive(Debug, Clone)]
pub struct VisualRefinement {
    /// Preprocessed image variants.
    pub prepared: PreparedVisionImage,
    /// Spatially grounded anchors.
    pub anchors: Vec<VisualAnchor>,
    /// Text-only anchors derived from visual anchors.
    pub text_anchors: Vec<TextAnchor>,
    /// Semantic overlay text for prompt preprending.
    pub semantic_overlay: Option<String>,
    /// `DeepSeek` OCR markdown truth, when available.
    pub ocr_truth_markdown: Option<String>,
}

impl VisualRefinement {
    /// Builds a visual `CoT` prompt bound to this refinement result.
    #[must_use]
    pub fn build_cot_prompt(&self, user_goal: &str, mode: VisualCotMode) -> String {
        let cot = build_visual_cot_prompt(VisualCotInput {
            anchors: self.text_anchors.clone(),
            mode,
        });
        let normalized_goal = user_goal.trim();
        if cot.is_empty() {
            normalized_goal.to_string()
        } else if normalized_goal.is_empty() {
            cot
        } else {
            format!("{cot}\n{normalized_goal}")
        }
    }
}

/// Vision refiner that applies deterministic preprocessing + grounding.
#[derive(Debug, Clone)]
pub struct VisualRefiner {
    max_dimension: u32,
    min_confidence: f32,
}

impl Default for VisualRefiner {
    fn default() -> Self {
        Self {
            max_dimension: DEFAULT_VISION_MAX_DIMENSION,
            min_confidence: 0.55,
        }
    }
}

impl VisualRefiner {
    /// Creates a vision refiner with explicit resizing and confidence policy.
    #[must_use]
    pub fn new(max_dimension: u32, min_confidence: f32) -> Self {
        Self {
            max_dimension: max_dimension.max(1),
            min_confidence: min_confidence.clamp(0.0, 1.0),
        }
    }

    /// Refines raw image bytes and produces semantic overlay artifacts.
    ///
    /// # Errors
    ///
    /// Returns an error when preprocessing or OCR truth extraction fails.
    pub fn refine(&self, image_bytes: Arc<[u8]>) -> LlmResult<VisualRefinement> {
        let prepared = preprocess_image_with_max_dimension(image_bytes, self.max_dimension)?;
        let runtime = get_deepseek_runtime();
        let ocr_truth_markdown = infer_deepseek_ocr_truth(runtime.as_ref(), &prepared)?;
        let anchors =
            self.detect_visual_anchors(runtime.as_ref(), &prepared, ocr_truth_markdown.as_deref());
        let text_anchors = anchors.iter().map(VisualAnchor::to_text_anchor).collect();
        let semantic_overlay = build_semantic_overlay(anchors.as_slice());
        Ok(VisualRefinement {
            prepared,
            anchors,
            text_anchors,
            semantic_overlay,
            ocr_truth_markdown,
        })
    }

    fn detect_visual_anchors(
        &self,
        runtime: &DeepseekRuntime,
        _prepared: &PreparedVisionImage,
        _ocr_truth_markdown: Option<&str>,
    ) -> Vec<VisualAnchor> {
        // DeepSeek runtime remains intentionally lazy-bound to model
        // configuration. If models are not configured, this stage is a no-op.
        if !runtime.is_enabled() {
            return Vec::new();
        }

        // Reserved hook for DeepSeek anchor extraction. Keep deterministic
        // filtering in the refiner so OCR backend output stays stable.
        let raw_anchors: Vec<VisualAnchor> = Vec::new();
        raw_anchors
            .into_iter()
            .filter(|anchor| anchor.confidence >= self.min_confidence)
            .collect()
    }
}

/// Builds a stable semantic overlay from visual anchors.
///
/// The format is designed for direct prompt injection ahead of image parts.
#[must_use]
pub fn build_semantic_overlay(anchors: &[VisualAnchor]) -> Option<String> {
    if anchors.is_empty() {
        return None;
    }

    let mut sorted: Vec<&VisualAnchor> = anchors
        .iter()
        .filter(|anchor| !anchor.text.trim().is_empty())
        .collect();
    if sorted.is_empty() {
        return None;
    }
    sorted.sort_by(|left, right| {
        right
            .confidence
            .partial_cmp(&left.confidence)
            .unwrap_or(Ordering::Equal)
    });

    let lines = sorted
        .into_iter()
        .map(|anchor| {
            let [x, y, w, h] = anchor.bbox;
            format!(
                "- text=\"{}\" confidence={:.2} bbox=[{x},{y},{w},{h}]",
                escape_overlay_text(anchor.text.as_ref()),
                anchor.confidence
            )
        })
        .collect::<Vec<_>>();

    Some(format!(
        "[vision-overlay]\n{}\n[/vision-overlay]",
        lines.join("\n")
    ))
}

fn escape_overlay_text(input: &str) -> String {
    input
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', " ")
        .trim()
        .to_string()
}
