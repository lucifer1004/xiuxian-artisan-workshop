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
fn test_discover_files() {
    use xiuxian_wendao::SyncEngine;

    let temp_dir = TempDir::new().unwrap();

    // Create test files
    fs::write(temp_dir.path().join("test.py"), "print('hello')").unwrap();
    fs::write(temp_dir.path().join("test.md"), "# Hello").unwrap();
    fs::write(temp_dir.path().join("test.txt"), "hello").unwrap(); // Should be skipped

    // Create subdirectory with file
    let subdir = temp_dir.path().join("subdir");
    fs::create_dir_all(&subdir).unwrap();
    fs::write(subdir.join("module.py"), "def foo(): pass").unwrap();

    let manifest_path = temp_dir.path().join("manifest.json");
    let engine = SyncEngine::new(temp_dir.path(), &manifest_path);
    let files = engine.discover_files();

    // Should find .py and .md files, not .txt
    assert!(
        files
            .iter()
            .any(|p| p.extension().map(|e| e == "py").unwrap_or(false))
    );
    assert!(
        files
            .iter()
            .any(|p| p.extension().map(|e| e == "md").unwrap_or(false))
    );
    assert!(
        !files
            .iter()
            .any(|p| p.extension().map(|e| e == "txt").unwrap_or(false))
    );
}
