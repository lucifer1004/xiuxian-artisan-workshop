//! Unified acceleration config tests.

use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use xiuxian_llm::llm::acceleration::AccelerationDevice;
use xiuxian_llm::test_support::{
    load_acceleration_device_with_paths, parse_acceleration_device_for_tests,
    resolve_acceleration_device_with_for_tests,
};

struct TempTree {
    root: PathBuf,
    config_home: PathBuf,
}

impl TempTree {
    fn new(label: &str) -> Self {
        let mut root = std::env::temp_dir();
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |duration| duration.as_nanos());
        root.push(format!(
            "xiuxian-llm-acceleration-{label}-{}-{nanos}",
            std::process::id()
        ));
        if let Err(error) = fs::create_dir_all(&root) {
            panic!(
                "failed to create test temp root {}: {error}",
                root.display()
            );
        }
        let config_home = root.join("config-home");
        if let Err(error) = fs::create_dir_all(&config_home) {
            panic!(
                "failed to create config home {}: {error}",
                config_home.display()
            );
        }
        Self { root, config_home }
    }

    fn xiuxian_toml_path(&self) -> PathBuf {
        self.config_home
            .join("xiuxian-artisan-workshop")
            .join("xiuxian.toml")
    }
}

impl Drop for TempTree {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

fn write_toml(path: &Path, content: &str) {
    if let Some(parent) = path.parent()
        && let Err(error) = fs::create_dir_all(parent)
    {
        panic!(
            "failed to create config parent directory {}: {error}",
            parent.display()
        );
    }
    if let Err(error) = fs::write(path, content) {
        panic!("failed to write config file {}: {error}", path.display());
    }
}

#[test]
fn parse_acceleration_device_supports_all_modes() {
    assert_eq!(
        parse_acceleration_device_for_tests(Some("auto")),
        Some(AccelerationDevice::Auto)
    );
    assert_eq!(
        parse_acceleration_device_for_tests(Some("cpu")),
        Some(AccelerationDevice::Cpu)
    );
    assert_eq!(
        parse_acceleration_device_for_tests(Some("metal")),
        Some(AccelerationDevice::Metal)
    );
    assert_eq!(
        parse_acceleration_device_for_tests(Some("cuda")),
        Some(AccelerationDevice::Cuda)
    );
    assert_eq!(parse_acceleration_device_for_tests(Some("unknown")), None);
}

#[test]
fn resolve_acceleration_device_uses_expected_precedence() {
    let mode = resolve_acceleration_device_with_for_tests(
        Some("cuda"),
        Some("cpu"),
        Some("metal"),
        Some("auto"),
    );
    assert_eq!(mode, AccelerationDevice::Cuda);

    let mode = resolve_acceleration_device_with_for_tests(None, Some("cpu"), Some("metal"), None);
    assert_eq!(mode, AccelerationDevice::Cpu);

    let mode = resolve_acceleration_device_with_for_tests(None, None, Some("metal"), None);
    assert_eq!(mode, AccelerationDevice::Metal);

    let mode = resolve_acceleration_device_with_for_tests(None, None, None, Some("cuda"));
    assert_eq!(mode, AccelerationDevice::Cuda);

    let mode = resolve_acceleration_device_with_for_tests(None, None, None, None);
    assert_eq!(mode, AccelerationDevice::Auto);
}

#[test]
fn load_acceleration_device_with_paths_reads_user_overlay() {
    let layout = TempTree::new("user-overlay");
    write_toml(
        layout.xiuxian_toml_path().as_path(),
        r#"
[llm.acceleration]
device = "metal"
"#,
    );
    let device = load_acceleration_device_with_paths(
        Some(layout.root.as_path()),
        Some(layout.config_home.as_path()),
    );
    assert_eq!(device.as_deref(), Some("metal"));
}
