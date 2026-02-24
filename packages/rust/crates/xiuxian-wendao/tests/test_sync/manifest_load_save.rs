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
fn test_manifest_load_save() {
    use xiuxian_wendao::SyncEngine;

    let temp_dir = TempDir::new().unwrap();
    let manifest_path = temp_dir.path().join("manifest.json");
    let engine = SyncEngine::new(temp_dir.path(), &manifest_path);

    // Create initial manifest
    let mut manifest = xiuxian_wendao::SyncManifest::default();
    manifest
        .0
        .insert("test.py".to_string(), "hash123".to_string());

    // Save and load
    engine.save_manifest(&manifest).unwrap();
    let loaded = engine.load_manifest();

    assert_eq!(loaded.0.get("test.py"), Some(&"hash123".to_string()));
}
