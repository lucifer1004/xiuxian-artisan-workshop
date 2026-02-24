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
fn test_batch_diff_computation() {
    use xiuxian_wendao::{SyncEngine, SyncManifest};

    let temp_dir = TempDir::new().unwrap();

    // Create many files
    for i in 0..50 {
        fs::write(
            temp_dir.path().join(format!("file_{}.py", i)),
            format!("content {}", i),
        )
        .unwrap();
    }

    let manifest_path = temp_dir.path().join("manifest.json");
    let engine = SyncEngine::new(temp_dir.path(), &manifest_path);

    // Empty manifest - all should be added
    let empty_manifest = SyncManifest::default();
    let files = engine.discover_files();
    let diff = engine.compute_diff(&empty_manifest, &files);

    // All 50 files should be added
    assert_eq!(diff.added.len(), 50);
    assert_eq!(diff.modified.len(), 0);
    assert_eq!(diff.unchanged, 0);
}
