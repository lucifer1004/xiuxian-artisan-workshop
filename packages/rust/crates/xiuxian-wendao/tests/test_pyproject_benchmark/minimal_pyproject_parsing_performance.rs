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
fn test_minimal_pyproject_parsing_performance() {
    let content = r#"
[project]
name = "test"
version = "0.1.0"
dependencies = [
    "requests>=2.0",
    "click>=8.0",
    "rich>=13.0",
    "typer>=0.9",
    "pydantic>=2.0",
]
"#;

    let start = std::time::Instant::now();

    // Parse the same content many times
    for _ in 0..100 {
        let mut file = NamedTempFile::new().unwrap();
        file.write_all(content.as_bytes()).unwrap();

        let deps = parse_pyproject_dependencies(file.path()).unwrap();
        assert_eq!(deps.len(), 5);
    }

    let elapsed = start.elapsed();

    // Should complete 100 parses in under 300ms
    let max_duration = std::time::Duration::from_millis(300);
    assert!(
        elapsed < max_duration,
        "Minimal pyproject parsing took {:.2}ms for 100 iterations, expected < 300ms",
        elapsed.as_secs_f64() * 1000.0
    );

    println!(
        "Minimal pyproject parsing: 100 iterations = {:.2}ms",
        elapsed.as_secs_f64() * 1000.0
    );
}
