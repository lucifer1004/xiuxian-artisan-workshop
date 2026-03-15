//! Shared JSON fixture assertions for `xiuxian-wendao` integration tests.
//!
//! Uses file-based snapshots in tests/snapshots/ directory.

use serde_json::Value;
use std::fs;
use std::path::PathBuf;

fn snapshot_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("snapshots")
}

fn render_json(value: &Value) -> String {
    serde_json::to_string_pretty(value).unwrap_or_else(|_| value.to_string())
}

/// Extracts content from snapshot format (skips metadata header)
fn extract_snapshot_content(content: &str) -> String {
    // Find the second --- which marks end of metadata
    let mut lines = content.lines();
    let mut found_first = false;

    for line in &mut lines {
        if line.starts_with("---") {
            if found_first {
                // Found second ---, content starts from here
                break;
            }
            found_first = true;
        }
    }

    // Collect remaining lines as content
    lines.collect::<Vec<_>>().join("\n").trim().to_string()
}

/// Asserts that actual JSON matches the expected snapshot.
///
/// Creates a new snapshot if one doesn't exist.
///
/// # Panics
/// Panics if the actual JSON differs from the snapshot.
pub fn assert_json_fixture_eq(fixture_root: &str, snapshot_name: &str, actual: &Value) {
    let snapshot_dir = snapshot_dir();
    fs::create_dir_all(&snapshot_dir).ok();

    // Normalize snapshot name (replace / with _)
    let safe_name = format!(
        "{}_{}",
        fixture_root.replace('/', "_"),
        snapshot_name.replace('/', "_")
    );
    let snapshot_file = snapshot_dir.join(format!("{}.snap", safe_name));

    let actual_json = render_json(actual);
    let actual_trimmed = actual_json.trim();

    if snapshot_file.exists() {
        let expected = fs::read_to_string(&snapshot_file).unwrap_or_default();
        let expected_content = extract_snapshot_content(&expected);

        if expected_content != actual_trimmed {
            // Write .snap.new file for review
            let new_file = snapshot_dir.join(format!("{}.snap.new", safe_name));
            let content = format!("---\nsource: test\nexpression: value\n---\n{}", actual_json);
            fs::write(&new_file, &content).ok();

            panic!(
                "Snapshot mismatch: {}\n\n--- expected ---\n{}\n\n--- actual ---\n{}\n\nNew snapshot written to: {}",
                safe_name,
                expected_content,
                actual_trimmed,
                new_file.display()
            );
        }
    } else {
        // Create new snapshot
        let content = format!("---\nsource: test\nexpression: value\n---\n{}", actual_json);
        fs::write(&snapshot_file, &content).unwrap();
        // Test passes on first run when creating snapshot
    }
}
