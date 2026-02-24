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
fn test_complex_dependency_parsing_performance() {
    // Test the complex format: name = { version = "x.y.z", features = [...] }
    let content = r#"
[package]
name = "test"
version = "0.1.0"

[dependencies]
tokio = { version = "1.49.0", features = ["full", "tracing"] }
serde = { version = "1.0.228", features = ["derive", "rc"] }
serde_json = { version = "1.0.149", features = ["std", "arbitrary_precision"] }
anyhow = { version = "1.0.100", features = ["backtrace"] }
thiserror = { version = "2.0.17", features = ["std"] }
async-trait = { version = "0.1.83", features = ["async-lift"] }
futures = { version = "0.3.31", features = ["async-await", "compat"] }
"#;

    let start = std::time::Instant::now();

    // Parse the same content multiple times
    for _ in 0..100 {
        let mut file = NamedTempFile::new().unwrap();
        file.write_all(content.as_bytes()).unwrap();

        let deps = parse_cargo_dependencies(file.path()).unwrap();
        assert_eq!(deps.len(), 7);
    }

    let elapsed = start.elapsed();

    // Should complete 100 parses in under 500ms
    let max_duration = std::time::Duration::from_millis(500);
    assert!(
        elapsed < max_duration,
        "Complex dependency parsing took {:.2}ms for 100 iterations, expected < 500ms",
        elapsed.as_secs_f64() * 1000.0
    );

    println!(
        "Complex dependency parsing: 100 iterations = {:.2}ms",
        elapsed.as_secs_f64() * 1000.0
    );
}
