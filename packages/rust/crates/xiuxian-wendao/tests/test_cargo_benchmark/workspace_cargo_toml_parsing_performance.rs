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
fn test_workspace_cargo_toml_parsing_performance() {
    const MEMBER_COUNT: usize = 50;
    const DEP_COUNT: usize = 100;

    let start = std::time::Instant::now();

    // Parse multiple workspace Cargo.toml files
    for _ in 0..10 {
        let content = generate_workspace_cargo_toml(MEMBER_COUNT, DEP_COUNT);
        let mut file = NamedTempFile::new().unwrap();
        file.write_all(content.as_bytes()).unwrap();

        let deps = parse_cargo_dependencies(file.path()).unwrap();
        assert!(!deps.is_empty());
    }

    let elapsed = start.elapsed();

    // Should parse 10 workspace files in under 1 second
    let max_duration = std::time::Duration::from_secs(1);
    assert!(
        elapsed < max_duration,
        "Workspace Cargo.toml parsing took {:.2}s for 10 files, expected < 1s",
        elapsed.as_secs_f64()
    );

    println!(
        "Workspace Cargo.toml parsing: 10 files x {} members x {} deps = {:.2}ms",
        MEMBER_COUNT,
        DEP_COUNT,
        elapsed.as_secs_f64() * 1000.0
    );
}
