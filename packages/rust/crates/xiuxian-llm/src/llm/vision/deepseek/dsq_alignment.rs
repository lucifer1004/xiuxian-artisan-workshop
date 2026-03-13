use deepseek_ocr_dsq::DsqTensorDType;

pub(in crate::llm::vision::deepseek) fn required_qoffset_alignment(dtype: DsqTensorDType) -> u64 {
    dtype
        .elem_size_bytes()
        .map(|size| size as u64)
        // Quantized payloads still require at least 2-byte alignment for candle mapping.
        .unwrap_or(2)
}
