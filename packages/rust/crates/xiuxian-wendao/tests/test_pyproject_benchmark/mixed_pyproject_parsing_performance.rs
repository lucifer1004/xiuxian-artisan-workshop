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
fn test_mixed_pyproject_parsing_performance() {
    const FILE_COUNT: usize = 30;

    let start = std::time::Instant::now();
    let mut total_deps = 0;

    for i in 0..FILE_COUNT {
        let dep_count = 25 + (i % 50); // Vary the number of deps

        let content = if i % 3 == 0 {
            generate_pyproject_toml_with_extras(dep_count)
        } else {
            generate_pyproject_toml(dep_count)
        };

        let mut file = NamedTempFile::new().unwrap();
        file.write_all(content.as_bytes()).unwrap();

        let deps = parse_pyproject_dependencies(file.path()).unwrap();
        total_deps += deps.len();
    }

    let elapsed = start.elapsed();

    // Should complete in under 2 seconds
    let max_duration = std::time::Duration::from_secs(2);
    assert!(
        elapsed < max_duration,
        "Mixed pyproject parsing took {:.2}s for {} files, expected < 2s",
        elapsed.as_secs_f64(),
        FILE_COUNT
    );

    println!(
        "Mixed pyproject parsing: {} files = {:.2}ms ({} total deps)",
        FILE_COUNT,
        elapsed.as_secs_f64() * 1000.0,
        total_deps
    );
}
