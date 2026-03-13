//! Test support module for xiuxian-skills.
//!
//! This module provides shared test utilities for skill scanner tests.

use std::fs;
use std::path::{Path, PathBuf};

use xiuxian_skills::SkillScanner;

/// Read fixture content from the fixtures directory.
pub fn read_fixture(relative: &str) -> String {
    let path = fixture_path(relative);
    fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("failed to read fixture {}: {error}", path.display()))
}

/// Get the path to a fixture file.
pub fn fixture_path(relative: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join(relative)
}

/// Write fixture content to a target path.
pub fn write_fixture_file(target: &Path, fixture_relative: &str) {
    if let Some(parent) = target.parent() {
        fs::create_dir_all(parent).expect("failed to create parent directory");
    }
    let content = read_fixture(fixture_relative);
    fs::write(target, content).expect("failed to write fixture file");
}

/// Sanitize a path in a string, replacing it with a placeholder.
pub fn sanitize_path(text: &str, skill_path: &Path) -> String {
    text.replace(skill_path.to_string_lossy().as_ref(), "<SKILL_PATH>")
}

/// Sanitize paths in a JSON value.
pub fn sanitize_json_paths(value: &mut serde_json::Value, target: &Path) {
    match value {
        serde_json::Value::String(text) => {
            *text = sanitize_path(text, target);
        }
        serde_json::Value::Array(items) => {
            for item in items {
                sanitize_json_paths(item, target);
            }
        }
        serde_json::Value::Object(map) => {
            for (_, child) in map.iter_mut() {
                sanitize_json_paths(child, target);
            }
        }
        _ => {}
    }
}

/// Canonicalize a JSON value (sort object keys recursively).
pub fn canonicalize_json(value: serde_json::Value) -> serde_json::Value {
    match value {
        serde_json::Value::Object(map) => {
            let mut entries: Vec<(String, serde_json::Value)> = map.into_iter().collect();
            entries.sort_by(|left, right| left.0.cmp(&right.0));
            let mut sorted = serde_json::Map::new();
            for (key, child) in entries {
                sorted.insert(key, canonicalize_json(child));
            }
            serde_json::Value::Object(sorted)
        }
        serde_json::Value::Array(values) => {
            serde_json::Value::Array(values.into_iter().map(canonicalize_json).collect())
        }
        scalar => scalar,
    }
}

/// Get the default skill structure.
pub fn default_structure() -> xiuxian_skills::SkillStructure {
    SkillScanner::default_structure()
}
