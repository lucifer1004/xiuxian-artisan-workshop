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
fn test_skip_hidden_and_directories() {
    use xiuxian_wendao::SyncEngine;

    let temp_dir = TempDir::new().unwrap();

    // Create hidden file/dir
    fs::write(temp_dir.path().join(".hidden.py"), "hidden").unwrap();
    fs::create_dir_all(temp_dir.path().join(".git")).unwrap();
    fs::write(temp_dir.path().join(".git").join("config"), "config").unwrap();

    // Create normal files
    fs::write(temp_dir.path().join("visible.py"), "visible").unwrap();

    let manifest_path = temp_dir.path().join("manifest.json");
    let engine = SyncEngine::new(temp_dir.path(), &manifest_path);
    let files = engine.discover_files();

    // Should not include hidden files (file name starts with .)
    assert!(!files.iter().any(|p| {
        p.file_name()
            .map(|n| n.to_string_lossy().starts_with('.'))
            .unwrap_or(false)
    }));
    // Should include visible file
    assert!(
        files
            .iter()
            .any(|p| p.file_name().map(|n| n == "visible.py").unwrap_or(false))
    );
}
