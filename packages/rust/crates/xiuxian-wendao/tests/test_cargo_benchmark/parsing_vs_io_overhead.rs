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
fn test_parsing_vs_io_overhead() {
    const DEP_COUNT: usize = 50;

    // Generate content once
    let content = generate_cargo_toml(DEP_COUNT);

    // Test pure parsing (multiple parses of same content)
    let parse_start = std::time::Instant::now();
    for _ in 0..50 {
        let mut file = NamedTempFile::new().unwrap();
        file.write_all(content.as_bytes()).unwrap();
        let _deps = parse_cargo_dependencies(file.path()).unwrap();
    }
    let parse_elapsed = parse_start.elapsed();

    // Report the breakdown
    println!(
        "Parsing {} deps: {:.2}ms for 50 iterations (includes I/O)",
        DEP_COUNT,
        parse_elapsed.as_secs_f64() * 1000.0
    );

    // Just verify it completes in reasonable time
    let max_duration = std::time::Duration::from_secs(2);
    assert!(
        parse_elapsed < max_duration,
        "Parsing took too long: {:.2}s",
        parse_elapsed.as_secs_f64()
    );
}
