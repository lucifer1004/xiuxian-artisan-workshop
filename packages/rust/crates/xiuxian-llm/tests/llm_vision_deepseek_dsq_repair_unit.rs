//! `DeepSeek` OCR DSQ repair unit tests.

use std::fs::File;
use std::io::Write;
use tempfile::tempdir;
use xiuxian_llm::llm::vision::deepseek::{
    repair_dsq_if_needed, snapshot_qoffset_alignment_with_for_tests,
};

#[test]
fn test_dsq_automatic_repair_of_unaligned_offsets() {
    let dir = tempdir().unwrap();
    let dsq_path = dir.path().join("unaligned.dsq");

    // 1. Construct a MALFORMED DSQ file with unaligned q_offset
    // Header: Magic(7), Version(4), CandleVersion(4+len), ModelId(4+len), Backend(4+len), DefaultDType(4), BlockSize(4), TensorCount(4)
    let mut file = File::create(&dsq_path).unwrap();
    file.write_all(b"DSQSNAP").unwrap();
    file.write_all(&1u32.to_le_bytes()).unwrap(); // Version

    write_string(&mut file, "0.9.0"); // Candle version
    write_string(&mut file, "test-model"); // Model ID
    write_string(&mut file, "metal"); // Backend

    file.write_all(&12u32.to_le_bytes()).unwrap(); // Default Q4K (DType 12)
    file.write_all(&256u32.to_le_bytes()).unwrap(); // Block size
    file.write_all(&1u32.to_le_bytes()).unwrap(); // Tensor count

    // Tensor Record: Name(4+len), OutDim(4), InDim(4), QDType(4), QOffset(8), QLen(8), BiasOffset(8), BiasLen(8), BiasDType(4)
    write_string(&mut file, "test_tensor");
    // Use small dimensions that match q_len: 2x2 F32 = 16 bytes
    file.write_all(&2u32.to_le_bytes()).unwrap(); // Out dim = 2
    file.write_all(&2u32.to_le_bytes()).unwrap(); // In dim = 2
    file.write_all(&0u32.to_le_bytes()).unwrap(); // F32 (DType 0) -> REQUIRES 4-BYTE ALIGNMENT

    // DELIBERATELY SET UNALIGNED OFFSET: Calculate the actual header size first,
    // then set an unaligned offset AFTER the header ends.
    // Header: Magic(7), Version(4), CandleVersion(4+5), ModelId(4+10), Backend(4+5),
    //         DefaultDType(4), BlockSize(4), TensorCount(4)
    // Tensor: Name(4+11), OutDim(4), InDim(4), QDType(4), QOffset(8), QLen(8),
    //         BiasOffset(8), BiasLen(8), BiasDType(4)
    let header_size =
        7 + 4 + (4 + 5) + (4 + 10) + (4 + 5) + 4 + 4 + 4 + (4 + 11) + 4 + 4 + 4 + 8 + 8 + 8 + 8 + 4;
    // Set unaligned offset to header_size + 1 (odd, not aligned for F32 which needs 4-byte alignment)
    let unaligned_offset = header_size as u64 + 1;

    file.write_all(&unaligned_offset.to_le_bytes()).unwrap(); // QOffset
    file.write_all(&16u64.to_le_bytes()).unwrap(); // QLen (2x2 F32 = 16 bytes)
    file.write_all(&0u64.to_le_bytes()).unwrap(); // BiasOffset
    file.write_all(&0u64.to_le_bytes()).unwrap(); // BiasLen
    file.write_all(&0u32.to_le_bytes()).unwrap(); // BiasDType

    // Write dummy data at unaligned offset
    file.write_all(&[0u8; 100]).unwrap();
    file.sync_all().unwrap();
    drop(file);

    // 2. Verify it is indeed unaligned for F32 (DType 0)
    assert!(
        !snapshot_qoffset_alignment_with_for_tests(unaligned_offset, 0),
        "Offset {} should be unaligned for F32",
        unaligned_offset
    );

    // 3. Trigger Repair
    let result = repair_dsq_if_needed(&dsq_path);

    // 4. Validate Result
    match result {
        xiuxian_llm::llm::vision::deepseek::DsqRepairResult::Repaired => {
            // SUCCESS! Now check the file
            let reader = deepseek_ocr_dsq::DsqReader::open(&dsq_path).unwrap();
            let record = reader.records().first().unwrap();

            assert!(
                record.q_offset % 4 == 0,
                "Repaired offset {} must be 4-byte aligned for F32",
                record.q_offset
            );
            assert!(
                snapshot_qoffset_alignment_with_for_tests(record.q_offset, 0),
                "Repaired offset {} should pass alignment check",
                record.q_offset
            );
        }
        other => panic!("Expected Repaired, got {:?}", other),
    }
}

#[test]
fn test_dsq_alignment_for_float_types() {
    assert!(snapshot_qoffset_alignment_with_for_tests(2, 1));
    assert!(!snapshot_qoffset_alignment_with_for_tests(1, 1));
    assert!(snapshot_qoffset_alignment_with_for_tests(2, 16));
    assert!(!snapshot_qoffset_alignment_with_for_tests(1, 16));
}

fn write_string(file: &mut File, s: &str) {
    let bytes = s.as_bytes();
    file.write_all(&(bytes.len() as u32).to_le_bytes()).unwrap();
    file.write_all(bytes).unwrap();
}
