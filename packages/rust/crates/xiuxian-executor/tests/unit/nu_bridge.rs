//! Tests for `NuSystemBridge`.

use std::fs;
use std::time::Duration;

use tempfile::TempDir;
use xiuxian_executor::{ActionType, ExecutorError, NuConfig, NuSystemBridge};

#[test]
fn classify_action_detects_observe_and_mutate_commands() {
    assert_eq!(NuSystemBridge::classify_action("ls ."), ActionType::Observe);
    assert_eq!(
        NuSystemBridge::classify_action("mkdir workspace"),
        ActionType::Mutate
    );
}

#[test]
fn with_config_preserves_custom_settings() {
    let config = NuConfig {
        nu_path: "custom-nu".to_string(),
        no_config: false,
        timeout: Duration::from_secs(5),
        enable_shellcheck: false,
        allowed_commands: vec!["ls".to_string()],
    };

    let bridge = NuSystemBridge::with_config(config.clone());
    assert_eq!(bridge.config().nu_path, config.nu_path);
    assert_eq!(bridge.config().no_config, config.no_config);
    assert_eq!(bridge.config().timeout, config.timeout);
    assert_eq!(bridge.config().enable_shellcheck, config.enable_shellcheck);
    assert_eq!(bridge.config().allowed_commands, config.allowed_commands);
}

#[test]
fn execute_uses_ls_fast_path_for_structured_observe_commands()
-> Result<(), Box<dyn std::error::Error>> {
    let temp = TempDir::new()?;
    fs::write(temp.path().join("note.txt"), "hello")?;

    let bridge = NuSystemBridge::new();
    let result = bridge.execute(&format!("ls {}", temp.path().display()), true)?;
    let rows = result
        .as_array()
        .ok_or("structured observe fast path should return an array")?;

    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0]["name"], "note.txt");
    assert_eq!(rows[0]["type"], "file");
    Ok(())
}

#[test]
fn execute_ls_fast_path_includes_hidden_entries_when_requested()
-> Result<(), Box<dyn std::error::Error>> {
    let temp = TempDir::new()?;
    fs::write(temp.path().join(".secret"), "hidden")?;

    let bridge = NuSystemBridge::new();
    let result = bridge.execute(&format!("ls --all {}", temp.path().display()), true)?;
    let rows = result
        .as_array()
        .ok_or("structured observe fast path should return an array")?;

    assert!(rows.iter().any(|row| row["name"] == ".secret"));
    Ok(())
}

#[test]
fn validate_safety_rejects_dangerous_patterns() {
    let bridge = NuSystemBridge::new();
    let error = match bridge.validate_safety("rm -rf /") {
        Ok(()) => panic!("dangerous pattern should be rejected"),
        Err(error) => error,
    };

    assert!(matches!(error, ExecutorError::SecurityViolation(_)));
}
