use image::DynamicImage;

use super::super::super::super::preprocess::PreparedVisionImage;

pub(super) fn decode_engine_input_image(prepared: &PreparedVisionImage) -> DynamicImage {
    image::load_from_memory(prepared.resized_png.as_ref())
        .expect("resized_png should be valid PNG from preprocessing")
}
