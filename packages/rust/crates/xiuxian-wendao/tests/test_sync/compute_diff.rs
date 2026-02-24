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
fn test_compute_diff() {
    use xiuxian_wendao::{SyncEngine, SyncManifest};

    let temp_dir = TempDir::new().unwrap();

    // Create test files
    fs::write(temp_dir.path().join("new.py"), "new content").unwrap();
    fs::write(temp_dir.path().join("modified.py"), "modified content").unwrap();
    fs::write(temp_dir.path().join("existing.py"), "existing").unwrap();

    let manifest_path = temp_dir.path().join("manifest.json");
    let engine = SyncEngine::new(temp_dir.path(), &manifest_path);

    // Create old manifest (existing unchanged, modified changed, new missing)
    let mut old_manifest = SyncManifest::default();
    old_manifest.0.insert(
        "existing.py".to_string(),
        SyncEngine::compute_hash("existing"),
    );
    old_manifest
        .0
        .insert("modified.py".to_string(), "old_hash".to_string()); // Different content

    let files = engine.discover_files();
    let diff = engine.compute_diff(&old_manifest, &files);

    // new.py should be in added
    assert!(
        diff.added
            .iter()
            .any(|p| p.file_name().map(|n| n == "new.py").unwrap_or(false))
    );

    // modified.py should be in modified
    assert!(
        diff.modified
            .iter()
            .any(|p| p.file_name().map(|n| n == "modified.py").unwrap_or(false))
    );

    // existing.py should be unchanged
    assert_eq!(diff.unchanged, 1);
}
