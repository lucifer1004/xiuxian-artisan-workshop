use image::DynamicImage;

use super::super::super::super::preprocess::PreparedVisionImage;

pub(super) fn decode_engine_input_image(prepared: &PreparedVisionImage) -> DynamicImage {
    prepared.decoded.as_ref().clone()
}
