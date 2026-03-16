use deepseek_ocr_dsq::DsqTensorDType;

pub(in crate::llm::vision::deepseek) fn required_qoffset_alignment(dtype: DsqTensorDType) -> u64 {
    dtype.elem_size_bytes().map_or(2, |size| size as u64)
}
