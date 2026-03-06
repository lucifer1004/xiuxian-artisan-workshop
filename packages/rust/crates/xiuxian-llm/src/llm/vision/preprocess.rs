use std::sync::Arc;

use fast_image_resize as fir;
use image::DynamicImage;
use image::GenericImageView;

use crate::llm::error::{LlmError, LlmResult};

/// Default longest-edge bound used by vision preprocessing.
pub const DEFAULT_VISION_MAX_DIMENSION: u32 = 2_048;

/// Preprocessed image payloads ready for multimodal/vision pipelines.
#[derive(Debug, Clone)]
pub struct PreparedVisionImage {
    /// Original input bytes.
    pub original: Arc<[u8]>,
    /// Decoded RGB image used by OCR inference (zero-copy handoff).
    pub decoded: Arc<DynamicImage>,
    /// Final image width after preprocessing.
    pub width: u32,
    /// Final image height after preprocessing.
    pub height: u32,
    /// Scale factor applied to the original dimensions.
    pub scale: f64,
}

/// Preprocess an image using the default max dimension bound.
///
/// # Errors
///
/// Returns an error when decoding or resize fails.
pub fn preprocess_image(image_bytes: Arc<[u8]>) -> LlmResult<PreparedVisionImage> {
    preprocess_image_with_max_dimension(image_bytes, DEFAULT_VISION_MAX_DIMENSION)
}

/// Preprocess an image with a custom longest-edge bound.
///
/// Steps:
/// 1. Decode image bytes.
/// 2. Resize to fit within `max_dimension` while preserving aspect ratio.
/// 3. Produce a grayscale variant for OCR preprocessing.
///
/// # Errors
///
/// Returns an error when decoding or resize fails.
pub fn preprocess_image_with_max_dimension(
    image_bytes: Arc<[u8]>,
    max_dimension: u32,
) -> LlmResult<PreparedVisionImage> {
    let decoded = image::load_from_memory(image_bytes.as_ref())
        .map_err(|error| internal_error(format!("vision image decode failed: {error}")))?;
    let (original_width, original_height) = decoded.dimensions();
    let (width, height, scale) = fit_dimensions(original_width, original_height, max_dimension);

    let resized = if width == original_width && height == original_height {
        decoded
    } else {
        resize_with_fast_image_resize(&decoded, width, height)?
    };

    let decoded = Arc::new(resized);

    Ok(PreparedVisionImage {
        original: image_bytes,
        decoded,
        width,
        height,
        scale,
    })
}

fn resize_with_fast_image_resize(
    source: &DynamicImage,
    width: u32,
    height: u32,
) -> LlmResult<DynamicImage> {
    let source_rgba = DynamicImage::ImageRgba8(source.to_rgba8());
    let mut target_rgba = DynamicImage::new_rgba8(width, height);
    let options = fir::ResizeOptions::new()
        .resize_alg(fir::ResizeAlg::Convolution(fir::FilterType::Lanczos3));
    let mut resizer = fir::Resizer::new();
    resizer
        .resize(&source_rgba, &mut target_rgba, Some(&options))
        .map_err(|error| internal_error(format!("vision image resize failed: {error}")))?;
    Ok(target_rgba)
}

impl PreparedVisionImage {
    /// Create a tiny in-memory image used for runtime warmup.
    #[must_use]
    pub fn create_dummy(width: u32, height: u32) -> Self {
        let width = width.max(1);
        let height = height.max(1);
        let decoded = DynamicImage::new_rgba8(width, height);
        Self {
            original: Arc::from(Vec::<u8>::new().into_boxed_slice()),
            decoded: Arc::new(decoded),
            width,
            height,
            scale: 1.0,
        }
    }
}

/// Computes resized dimensions that fit within `max_dimension` while preserving
/// aspect ratio.
#[must_use]
pub fn fit_dimensions(width: u32, height: u32, max_dimension: u32) -> (u32, u32, f64) {
    if width == 0 || height == 0 {
        return (1, 1, 1.0);
    }
    if max_dimension == 0 {
        return (width, height, 1.0);
    }

    let long_edge = width.max(height);
    if long_edge <= max_dimension {
        return (width, height, 1.0);
    }

    let scale = f64::from(max_dimension) / f64::from(long_edge);
    let target_width = fit_edge_with_rounding(width, long_edge, max_dimension);
    let target_height = fit_edge_with_rounding(height, long_edge, max_dimension);
    (target_width, target_height, scale)
}

#[inline]
#[must_use]
fn fit_edge_with_rounding(edge: u32, long_edge: u32, max_dimension: u32) -> u32 {
    let numerator = u64::from(edge) * u64::from(max_dimension);
    let denominator = u64::from(long_edge);
    let rounded = (numerator + (denominator / 2)) / denominator;
    let bounded = rounded.max(1).min(u64::from(u32::MAX));
    u32::try_from(bounded).unwrap_or(u32::MAX)
}

fn internal_error(message: String) -> LlmError {
    LlmError::Internal { message }
}
