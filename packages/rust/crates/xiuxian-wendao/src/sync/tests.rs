#![allow(clippy::expect_used, clippy::map_unwrap_or)]

use std::collections::HashMap;
use std::fs;

use tempfile::TempDir;

use super::{SyncEngine, SyncManifest};

#[test]
fn test_manifest_load_save() {
    let temp_dir = TempDir::new().expect("create temp dir");
    let manifest_path = temp_dir.path().join("manifest.json");
    let engine = SyncEngine::new(temp_dir.path(), &manifest_path);

    // Create initial manifest
    let mut manifest = SyncManifest(HashMap::new());
    manifest
        .0
        .insert("test.py".to_string(), "hash123".to_string());

    // Save and load
    engine.save_manifest(&manifest).expect("save manifest");
    let loaded = engine.load_manifest();

    assert_eq!(loaded.0.get("test.py"), Some(&"hash123".to_string()));
}

#[test]
fn test_compute_hash() {
    let hash1 = SyncEngine::compute_hash("hello world");
    let hash2 = SyncEngine::compute_hash("hello world");
    let hash3 = SyncEngine::compute_hash("different");

    assert_eq!(hash1, hash2);
    assert_ne!(hash1, hash3);
    // xxhash produces 16 character hex
    assert_eq!(hash1.len(), 16);
}

#[test]
fn test_discover_files() {
    let temp_dir = TempDir::new().expect("create temp dir");

    // Create test files
    fs::write(temp_dir.path().join("test.py"), "print('hello')").expect("write");
    fs::write(temp_dir.path().join("test.md"), "# Hello").expect("write");
    fs::write(temp_dir.path().join("test.txt"), "hello").expect("write"); // Should be skipped

    // Create subdirectory with file
    let subdir = temp_dir.path().join("subdir");
    fs::create_dir_all(&subdir).expect("create subdir");
    fs::write(subdir.join("module.py"), "def foo(): pass").expect("write");

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

#[test]
fn test_compute_diff() {
    let temp_dir = TempDir::new().expect("create temp dir");

    // Create test files
    fs::write(temp_dir.path().join("new.py"), "new content").expect("write");
    fs::write(temp_dir.path().join("modified.py"), "modified content").expect("write");
    fs::write(temp_dir.path().join("existing.py"), "existing").expect("write");

    let manifest_path = temp_dir.path().join("manifest.json");
    let engine = SyncEngine::new(temp_dir.path(), &manifest_path);

    // Create old manifest (existing.py unchanged, modified.py changed, missing new.py)
    let mut old_manifest = SyncManifest(HashMap::new());
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

#[test]
fn test_skip_hidden_and_directories() {
    let temp_dir = TempDir::new().expect("create temp dir");

    // Create hidden file/dir
    fs::write(temp_dir.path().join(".hidden.py"), "hidden").expect("write");
    fs::create_dir_all(temp_dir.path().join(".git")).expect("create git dir");
    fs::write(temp_dir.path().join(".git").join("config"), "config").expect("write");

    // Create normal files
    fs::write(temp_dir.path().join("visible.py"), "visible").expect("write");

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
