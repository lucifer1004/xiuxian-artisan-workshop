//! `DeepSeek` OCR snapshot path resolution tests.

use std::fs;
use std::path::Path;

use tempfile::tempdir;
use xiuxian_llm::test_support::resolve_deepseek_snapshot_path_for_tests;

fn write_file(path: &Path, payload: &[u8]) {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).unwrap_or_else(|error| {
            panic!("failed to create directory {}: {error}", parent.display())
        });
    }
    fs::write(path, payload)
        .unwrap_or_else(|error| panic!("failed to write file {}: {error}", path.display()));
}

#[test]
fn snapshot_resolution_rejects_ambiguous_auto_detect() {
    let dir = tempdir().expect("temporary directory should be created");
    write_file(&dir.path().join("dots.ocr.Q4_K.dsq"), b"q4");
    write_file(&dir.path().join("dots.ocr.Q6_K.dsq"), b"q6");

    let resolved = resolve_deepseek_snapshot_path_for_tests(dir.path(), None);

    assert_eq!(resolved, None);
}

#[test]
fn snapshot_resolution_honors_explicit_override_in_ambiguous_directory() {
    let dir = tempdir().expect("temporary directory should be created");
    let q4 = dir.path().join("dots.ocr.Q4_K.dsq");
    let q6 = dir.path().join("dots.ocr.Q6_K.dsq");
    write_file(&q4, b"q4");
    write_file(&q6, b"q6");

    let resolved =
        resolve_deepseek_snapshot_path_for_tests(dir.path(), Some(q6.to_string_lossy().as_ref()));

    assert_eq!(resolved, Some(q6.display().to_string()));
}

#[test]
fn snapshot_resolution_returns_single_auto_detected_snapshot() {
    let dir = tempdir().expect("temporary directory should be created");
    let snapshot = dir.path().join("dots.ocr.Q6_K.dsq");
    write_file(&snapshot, b"q6");

    let resolved = resolve_deepseek_snapshot_path_for_tests(dir.path(), None);

    assert_eq!(resolved, Some(snapshot.display().to_string()));
}
