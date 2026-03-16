use image::DynamicImage;

use crate::llm::vision::PreparedVisionImage;

pub(super) fn decode_engine_input_image(prepared: &PreparedVisionImage) -> DynamicImage {
    image::load_from_memory(prepared.resized_png.as_ref())
        .unwrap_or_else(|_| DynamicImage::ImageRgb8(image::ImageBuffer::new(1, 1)))
}
