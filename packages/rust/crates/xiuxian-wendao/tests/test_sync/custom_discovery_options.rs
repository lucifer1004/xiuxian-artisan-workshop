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
fn test_custom_discovery_options() {
    use xiuxian_wendao::{DiscoveryOptions, SyncEngine};

    let temp_dir = TempDir::new().unwrap();

    // Create files with different extensions
    fs::write(temp_dir.path().join("test.rs"), "fn main() {}").unwrap();
    fs::write(temp_dir.path().join("test.go"), "package main").unwrap();

    let manifest_path = temp_dir.path().join("manifest.json");

    // Create engine with custom options (only .rs files)
    let options = DiscoveryOptions {
        extensions: vec!["rs".to_string()],
        ..Default::default()
    };

    let engine = SyncEngine::new(temp_dir.path(), &manifest_path).with_options(options);
    let files = engine.discover_files();

    // Should only find .rs file
    assert_eq!(files.len(), 1);
    assert!(files[0].extension().map(|e| e == "rs").unwrap_or(false));
}
