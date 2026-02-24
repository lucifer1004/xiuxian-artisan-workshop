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
fn test_deleted_files_detection() {
    use xiuxian_wendao::{SyncEngine, SyncManifest};

    let temp_dir = TempDir::new().unwrap();
    let manifest_path = temp_dir.path().join("manifest.json");
    let engine = SyncEngine::new(temp_dir.path(), &manifest_path);

    // Create old manifest with files that don't exist on disk
    let mut old_manifest = SyncManifest::default();
    old_manifest
        .0
        .insert("deleted1.py".to_string(), "hash1".to_string());
    old_manifest
        .0
        .insert("deleted2.rs".to_string(), "hash2".to_string());
    old_manifest.0.insert(
        "still_exists.py".to_string(),
        SyncEngine::compute_hash("exists"),
    );

    // Create file for still_exists
    fs::write(temp_dir.path().join("still_exists.py"), "exists").unwrap();

    let files = engine.discover_files();
    let diff = engine.compute_diff(&old_manifest, &files);

    // deleted1.py should be in deleted
    assert!(
        diff.deleted
            .iter()
            .any(|p| p.file_name().map(|n| n == "deleted1.py").unwrap_or(false))
    );
    // deleted2.rs should be in deleted
    assert!(
        diff.deleted
            .iter()
            .any(|p| p.file_name().map(|n| n == "deleted2.rs").unwrap_or(false))
    );
}
