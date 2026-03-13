use std::io::Cursor;
use std::sync::Arc;

use image::GenericImageView;
use image::ImageFormat;
use image::imageops::FilterType;
use imageproc::contrast::equalize_histogram;

use crate::llm::error::{LlmError, LlmResult};

/// Default longest-edge bound used by vision preprocessing.
pub const DEFAULT_VISION_MAX_DIMENSION: u32 = 2_048;

/// Preprocessed image payloads ready for multimodal/vision pipelines.
#[derive(Debug, Clone)]
pub struct PreparedVisionImage {
    /// Original input bytes.
    pub original: Arc<[u8]>,
    /// Resized RGB payload encoded as PNG.
    pub resized_png: Arc<[u8]>,
    /// Equalized grayscale payload encoded as PNG.
    pub grayscale_png: Arc<[u8]>,
    /// Final image width after preprocessing.
    pub width: u32,
    /// Final image height after preprocessing.
    pub height: u32,
    /// Scale factor applied to the original dimensions.
    pub scale: f64,
}

impl PreparedVisionImage {
    /// Create a dummy prepared image for prewarm/testing.
    pub fn create_dummy(width: u32, height: u32) -> Self {
        let dummy_data: Arc<[u8]> = Arc::from(vec![0u8; 0]);
        Self {
            original: dummy_data.clone(),
            resized_png: dummy_data.clone(),
            grayscale_png: dummy_data,
            width,
            height,
            scale: 1.0,
        }
    }
}

/// Preprocess an image using the default max dimension bound.
///
/// # Errors
///
/// Returns an error when decoding or PNG encoding fails.
pub fn preprocess_image(image_bytes: Arc<[u8]>) -> LlmResult<PreparedVisionImage> {
    preprocess_image_with_max_dimension(image_bytes, DEFAULT_VISION_MAX_DIMENSION)
}

/// Preprocess an image with a custom longest-edge bound.
///
/// Steps:
/// 1. Decode image bytes.
/// 2. Resize to fit within `max_dimension` while preserving aspect ratio.
/// 3. Produce an equalized grayscale variant for OCR-friendly contrast.
///
/// # Errors
///
/// Returns an error when decoding or PNG encoding fails.
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
        decoded.resize_exact(width, height, FilterType::Lanczos3)
    };

    let resized_png = encode_png(&resized)?;
    let grayscale = equalize_histogram(&resized.to_luma8());
    let grayscale_png = encode_png(&image::DynamicImage::ImageLuma8(grayscale))?;

    Ok(PreparedVisionImage {
        original: image_bytes,
        resized_png,
        grayscale_png,
        width,
        height,
        scale,
    })
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

pub fn encode_png(image: &image::DynamicImage) -> LlmResult<Arc<[u8]>> {
    let mut writer = Cursor::new(Vec::new());
    image
        .write_to(&mut writer, image::ImageFormat::Png)
        .map_err(|error| internal_error(format!("vision png encode failed: {error}")))?;
    Ok(Arc::from(writer.into_inner()))
}

fn internal_error(message: String) -> LlmError {
    LlmError::Internal { message }
}
