use std::fs::File;
use std::io::{BufWriter, Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};

use byteorder::{LittleEndian, WriteBytesExt};
use deepseek_ocr_dsq::DsqReader;

use crate::llm::vision::deepseek::dsq_alignment::required_qoffset_alignment;

/// Result of a DSQ repair attempt.
#[derive(Debug)]
pub enum DsqRepairResult {
    /// File was already aligned, no changes needed.
    AlreadyAligned,
    /// File was successfully repaired and replaced.
    Repaired,
    /// Repair failed with a specific error message.
    Failed(String),
}

pub fn repair_dsq_if_needed(path: &Path) -> DsqRepairResult {
    match validate_alignment(path) {
        Ok(true) => DsqRepairResult::AlreadyAligned,
        Ok(false) => perform_repair(path),
        Err(err) => DsqRepairResult::Failed(format!("Initial validation failed: {}", err)),
    }
}

fn validate_alignment(path: &Path) -> Result<bool, String> {
    let reader = DsqReader::open(path).map_err(|e| e.to_string())?;
    for record in reader.records() {
        let alignment = required_qoffset_alignment(record.q_dtype);
        if record.q_offset % alignment != 0 {
            return Ok(false);
        }
        if let Some(bias_offset) = record.bias_offset {
            let bias_alignment = 4;
            if bias_offset % bias_alignment != 0 {
                return Ok(false);
            }
        }
    }
    Ok(true)
}

fn perform_repair(path: &Path) -> DsqRepairResult {
    let temp_path = path.with_extension("dsq.repairing");

    let result = (|| -> Result<(), String> {
        let reader = DsqReader::open(path).map_err(|e| e.to_string())?;
        let output_file = File::create(&temp_path).map_err(|e| e.to_string())?;
        let mut writer = BufWriter::new(output_file);

        let mut new_records = reader.records().to_vec();
        let header = reader.header();

        let header_size = estimate_header_size(header, reader.records());
        let mut current_offset = header_size;

        for record in &mut new_records {
            let q_alignment = required_qoffset_alignment(record.q_dtype);
            current_offset = align_up(current_offset, q_alignment);
            record.q_offset = current_offset;
            current_offset += record.q_len;

            if let Some(bias_len) = record.bias_len {
                let bias_alignment = 4;
                current_offset = align_up(current_offset, bias_alignment);
                record.bias_offset = Some(current_offset);
                current_offset += bias_len;
            }
        }

        write_header_internal(&mut writer, header, &new_records)?;

        let mut input_file = File::open(path).map_err(|e| e.to_string())?;
        for (i, original_record) in reader.records().iter().enumerate() {
            let new_record = &new_records[i];

            pad_to_offset(&mut writer, new_record.q_offset)?;
            copy_range_internal(
                &mut input_file,
                &mut writer,
                original_record.q_offset,
                original_record.q_len,
            )?;

            if let (Some(old_off), Some(new_off), Some(len)) = (
                original_record.bias_offset,
                new_record.bias_offset,
                original_record.bias_len,
            ) {
                pad_to_offset(&mut writer, new_off)?;
                copy_range_internal(&mut input_file, &mut writer, old_off, len)?;
            }
        }

        writer.flush().map_err(|e| e.to_string())?;
        Ok(())
    })();

    match result {
        Ok(_) => {
            if let Err(e) = std::fs::rename(&temp_path, path) {
                let _ = std::fs::remove_file(&temp_path);
                DsqRepairResult::Failed(format!("Atomic swap failed: {}", e))
            } else {
                DsqRepairResult::Repaired
            }
        }
        Err(e) => {
            let _ = std::fs::remove_file(&temp_path);
            DsqRepairResult::Failed(e)
        }
    }
}

fn align_up(offset: u64, alignment: u64) -> u64 {
    if alignment <= 1 {
        return offset;
    }
    (offset + alignment - 1) / alignment * alignment
}

fn estimate_header_size(
    header: &deepseek_ocr_dsq::DsqHeader,
    records: &[deepseek_ocr_dsq::DsqRecord],
) -> u64 {
    let mut size = 7;
    size += 4;
    size += 4 + header.candle_version.len() as u64;
    size += 4 + header.model_id.len() as u64;
    size += 4 + header.backend.len() as u64;
    size += 4;
    size += 4;
    size += 4;
    for r in records {
        size += 4 + r.name.len() as u64;
        size += 4 + 4 + 4;
        size += 8 + 8 + 8 + 8;
        size += 4;
    }
    size
}

fn write_header_internal<W: Write>(
    writer: &mut W,
    header: &deepseek_ocr_dsq::DsqHeader,
    records: &[deepseek_ocr_dsq::DsqRecord],
) -> Result<(), String> {
    writer.write_all(b"DSQSNAP").map_err(|e| e.to_string())?;
    writer
        .write_u32::<LittleEndian>(header.version)
        .map_err(|e| e.to_string())?;
    write_string_internal(writer, &header.candle_version)?;
    write_string_internal(writer, &header.model_id)?;
    write_string_internal(writer, &header.backend)?;
    writer
        .write_u32::<LittleEndian>(header.default_qdtype.as_u32())
        .map_err(|e| e.to_string())?;
    writer
        .write_u32::<LittleEndian>(header.block_size)
        .map_err(|e| e.to_string())?;
    writer
        .write_u32::<LittleEndian>(records.len() as u32)
        .map_err(|e| e.to_string())?;

    for r in records {
        write_string_internal(writer, &r.name)?;
        writer
            .write_u32::<LittleEndian>(r.out_dim as u32)
            .map_err(|e| e.to_string())?;
        writer
            .write_u32::<LittleEndian>(r.in_dim as u32)
            .map_err(|e| e.to_string())?;
        writer
            .write_u32::<LittleEndian>(r.q_dtype.as_u32())
            .map_err(|e| e.to_string())?;
        writer
            .write_u64::<LittleEndian>(r.q_offset)
            .map_err(|e| e.to_string())?;
        writer
            .write_u64::<LittleEndian>(r.q_len)
            .map_err(|e| e.to_string())?;
        writer
            .write_u64::<LittleEndian>(r.bias_offset.unwrap_or(0))
            .map_err(|e| e.to_string())?;
        writer
            .write_u64::<LittleEndian>(r.bias_len.unwrap_or(0))
            .map_err(|e| e.to_string())?;

        let bias_dtype_code = match r.bias_dtype {
            Some(dt) => dt.as_u32(),
            None => 0,
        };
        writer
            .write_u32::<LittleEndian>(bias_dtype_code)
            .map_err(|e| e.to_string())?;
    }
    Ok(())
}

fn write_string_internal<W: Write>(writer: &mut W, s: &str) -> Result<(), String> {
    let bytes = s.as_bytes();
    writer
        .write_u32::<LittleEndian>(bytes.len() as u32)
        .map_err(|e| e.to_string())?;
    writer.write_all(bytes).map_err(|e| e.to_string())?;
    Ok(())
}

fn pad_to_offset<W: Write + Seek>(writer: &mut W, target_offset: u64) -> Result<(), String> {
    let current = writer.stream_position().map_err(|e| e.to_string())?;
    if current > target_offset {
        return Err(format!(
            "Writer position {} already passed target offset {}",
            current, target_offset
        ));
    }
    if target_offset > current {
        let padding = vec![0u8; (target_offset - current) as usize];
        writer.write_all(&padding).map_err(|e| e.to_string())?;
    }
    Ok(())
}

fn copy_range_internal<R: Read + Seek, W: Write>(
    reader: &mut R,
    writer: &mut W,
    offset: u64,
    len: u64,
) -> Result<(), String> {
    reader
        .seek(std::io::SeekFrom::Start(offset))
        .map_err(|e| e.to_string())?;
    let mut remaining = len;
    let mut buffer = [0u8; 8 * 1024 * 1024];
    while remaining > 0 {
        let to_read = remaining.min(buffer.len() as u64);
        reader
            .read_exact(&mut buffer[..to_read as usize])
            .map_err(|e| e.to_string())?;
        writer
            .write_all(&buffer[..to_read as usize])
            .map_err(|e| e.to_string())?;
        remaining -= to_read;
    }
    Ok(())
}
