#![allow(
    missing_docs,
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::doc_markdown,
    clippy::implicit_clone,
    clippy::uninlined_format_args,
    clippy::float_cmp,
    clippy::cast_lossless,
    clippy::cast_precision_loss,
    clippy::cast_sign_loss,
    clippy::cast_possible_truncation,
    clippy::manual_string_new,
    clippy::needless_raw_string_hashes,
    clippy::format_push_string,
    clippy::map_unwrap_or,
    clippy::unnecessary_to_owned,
    clippy::too_many_lines
)]
use super::*;

#[test]
fn test_mixed_symbol_extraction_performance() {
    const TOTAL_FILES: usize = 100; // 50 Rust + 50 Python

    let start = std::time::Instant::now();

    let mut all_symbols = Vec::new();

    // Process Rust files
    for _ in 0..(TOTAL_FILES / 2) {
        let content = generate_rust_test_file(250);
        let mut file = NamedTempFile::new().unwrap();
        file.write_all(content.as_bytes()).unwrap();
        let symbols = extract_symbols(&file.path().to_path_buf(), "rust").unwrap();
        all_symbols.extend(symbols);
    }

    // Process Python files
    for _ in 0..(TOTAL_FILES / 2) {
        let content = generate_python_test_file(250);
        let mut file = NamedTempFile::new().unwrap();
        file.write_all(content.as_bytes()).unwrap();
        let symbols = extract_symbols(&file.path().to_path_buf(), "python").unwrap();
        all_symbols.extend(symbols);
    }

    let elapsed = start.elapsed();

    // Performance assertion
    let max_duration = std::time::Duration::from_secs(3);
    assert!(
        elapsed < max_duration,
        "Mixed symbol extraction took {:.2}s, expected < 3s",
        elapsed.as_secs_f64()
    );

    println!(
        "Mixed symbol extraction: {} files = {:.2}ms ({} symbols)",
        TOTAL_FILES,
        elapsed.as_secs_f64() * 1000.0,
        all_symbols.len()
    );
}
