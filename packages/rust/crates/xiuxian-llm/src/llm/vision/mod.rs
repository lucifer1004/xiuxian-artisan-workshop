//! Vision preprocessing and semantic grounding for multimodal LLM requests.

mod anchor;
mod cot;
pub(crate) mod deepseek;
mod message;
mod preprocess;
mod refiner;
mod scrub;

pub use anchor::{TextAnchor, VisualAnchor};
pub use cot::{VisualCotInput, VisualCotMode, build_visual_cot_prompt};
pub use deepseek::{
    DeepseekRuntime, get_deepseek_runtime, infer_deepseek_ocr_truth, preprocess_image_for_ocr,
    prewarm_deepseek_ocr,
};
pub use message::{
    build_visual_user_message, build_visual_user_message_from_refinement,
    build_visual_user_message_with_ocr_truth,
};
pub use preprocess::{
    DEFAULT_VISION_MAX_DIMENSION, PreparedVisionImage, fit_dimensions, preprocess_image,
    preprocess_image_with_max_dimension,
};
pub use refiner::{VisualRefinement, VisualRefiner, build_semantic_overlay};
pub use scrub::{VisibilityScrubPolicy, scrub_text_anchors};
