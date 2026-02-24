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
fn test_python_symbol_extraction_performance() {
    const FILE_COUNT: usize = 50;
    const LINES_PER_FILE: usize = 500;

    let start = std::time::Instant::now();

    // Create and process multiple test files
    let mut temp_files = Vec::new();
    let mut all_symbols = Vec::new();

    for _ in 0..FILE_COUNT {
        let content = generate_python_test_file(LINES_PER_FILE);

        let mut file = NamedTempFile::new().unwrap();
        file.write_all(content.as_bytes()).unwrap();
        let path = file.path().to_path_buf();

        let symbols = extract_symbols(&path, "python").unwrap();
        all_symbols.extend(symbols);
        temp_files.push(file);
    }

    let elapsed = start.elapsed();

    // Verify we extracted a reasonable number of symbols
    assert!(!all_symbols.is_empty(), "Should extract symbols");

    // Performance assertion
    let max_duration = std::time::Duration::from_secs(2);
    assert!(
        elapsed < max_duration,
        "Python symbol extraction took {:.2}s, expected < 2s",
        elapsed.as_secs_f64()
    );

    println!(
        "Python symbol extraction: {} files x {} lines = {:.2}ms ({} symbols extracted)",
        FILE_COUNT,
        LINES_PER_FILE,
        elapsed.as_secs_f64() * 1000.0,
        all_symbols.len()
    );
}
