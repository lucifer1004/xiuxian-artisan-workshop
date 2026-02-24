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
fn test_regex_fallback_parsing_performance() {
    // This tests the regex fallback path (when TOML parsing fails)
    let content =
        "package1==1.0.0\npackage2>=2.0.0\npackage3~=4.0.0\nanother_package[extra]==5.0.0\n";

    let start = std::time::Instant::now();

    for _ in 0..100 {
        let mut file = NamedTempFile::new().unwrap();
        file.write_all(content.as_bytes()).unwrap();

        let deps = parse_pyproject_dependencies(file.path()).unwrap();
        assert_eq!(deps.len(), 4);
    }

    let elapsed = start.elapsed();

    // Should complete 100 parses in under 200ms
    let max_duration = std::time::Duration::from_millis(200);
    assert!(
        elapsed < max_duration,
        "Regex fallback parsing took {:.2}ms for 100 iterations, expected < 200ms",
        elapsed.as_secs_f64() * 1000.0
    );

    println!(
        "Regex fallback parsing: 100 iterations = {:.2}ms",
        elapsed.as_secs_f64() * 1000.0
    );
}
