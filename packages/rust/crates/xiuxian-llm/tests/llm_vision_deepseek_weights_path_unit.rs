//! `DeepSeek`/Dots OCR weights path resolution tests.

use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use xiuxian_llm::test_support::resolve_deepseek_weights_path_for_tests;

struct TempTree {
    root: PathBuf,
}

impl TempTree {
    fn new(label: &str) -> Self {
        let mut root = std::env::temp_dir();
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |duration| duration.as_nanos());
        root.push(format!(
            "xiuxian-llm-deepseek-weights-{label}-{}-{nanos}",
            std::process::id()
        ));
        if let Err(error) = fs::create_dir_all(&root) {
            panic!(
                "failed to create test temp root {}: {error}",
                root.display()
            );
        }
        Self { root }
    }
}

impl Drop for TempTree {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

fn write_file(path: &Path, payload: &[u8]) {
    if let Some(parent) = path.parent()
        && let Err(error) = fs::create_dir_all(parent)
    {
        panic!("failed to create directory {}: {error}", parent.display());
    }
    if let Err(error) = fs::write(path, payload) {
        panic!("failed to write file {}: {error}", path.display());
    }
}

#[test]
fn dots_weights_resolution_prefers_safetensors_index() {
    let layout = TempTree::new("dots-prefers-index");
    write_file(
        layout.root.join("model.safetensors.index.json").as_path(),
        br#"{"weight_map":{"layer":"model-00001-of-00002.safetensors"}}"#,
    );
    write_file(
        layout
            .root
            .join("model-00001-of-00002.safetensors")
            .as_path(),
        b"shard-1",
    );

    let resolved =
        resolve_deepseek_weights_path_for_tests(layout.root.as_path(), Some("dots"), None)
            .expect("dots weights path should resolve");

    assert!(resolved.ends_with("model.safetensors.index.json"));
}

#[test]
fn dots_weights_resolution_accepts_directory_override() {
    let layout = TempTree::new("dots-dir-override");
    let override_dir = layout.root.join("weights");
    write_file(
        override_dir.join("model.safetensors.index.json").as_path(),
        br#"{"weight_map":{"layer":"model-00001-of-00002.safetensors"}}"#,
    );
    write_file(
        override_dir
            .join("model-00001-of-00002.safetensors")
            .as_path(),
        b"shard-1",
    );

    let resolved = resolve_deepseek_weights_path_for_tests(
        layout.root.as_path(),
        Some("dots"),
        Some(override_dir.to_string_lossy().as_ref()),
    )
    .expect("dots override directory should resolve index path");

    assert!(resolved.ends_with("model.safetensors.index.json"));
}
