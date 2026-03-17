//! Integration tests for dynamic system prompt template loading.

use serde_json::json;
use std::fs;
use xiuxian_qianhuan::{ManifestationManager, ManifestationTemplateTarget};

#[test]
fn manifestation_manager_supports_runtime_template_override_directory() {
    let temp = tempfile::tempdir().unwrap_or_else(|error| panic!("create temp dir: {error}"));
    let override_dir = temp.path().join("override_templates");
    fs::create_dir_all(&override_dir)
        .unwrap_or_else(|error| panic!("create override template dir: {error}"));
    fs::write(
        override_dir.join("daily_agenda.md"),
        "Agenda override: {{ task }}",
    )
    .unwrap_or_else(|error| panic!("write override template: {error}"));

    let glob = format!("{}/*", override_dir.display());
    let manager = ManifestationManager::new(&[glob.as_str()])
        .unwrap_or_else(|error| panic!("create manifestation manager: {error}"));
    let rendered = manager
        .render_target(
            &ManifestationTemplateTarget::DailyAgenda,
            json!({ "task": "dynamic-history" }),
        )
        .unwrap_or_else(|error| panic!("render override template: {error}"));

    assert_eq!(rendered, "Agenda override: dynamic-history");
}

#[test]
fn manifestation_manager_hot_reloads_template_without_restart() {
    let temp = tempfile::tempdir().unwrap_or_else(|error| panic!("create temp dir: {error}"));
    let override_dir = temp.path().join("hot_reload_templates");
    fs::create_dir_all(&override_dir)
        .unwrap_or_else(|error| panic!("create override template dir: {error}"));
    let template_path = override_dir.join("daily_agenda.md");

    fs::write(&template_path, "Agenda v1: {{ task }}")
        .unwrap_or_else(|error| panic!("write v1 template: {error}"));

    let glob = format!("{}/*", override_dir.display());
    let manager = ManifestationManager::new(&[glob.as_str()])
        .unwrap_or_else(|error| panic!("create manifestation manager: {error}"));

    let first = manager
        .render_target(
            &ManifestationTemplateTarget::DailyAgenda,
            json!({ "task": "history-v1" }),
        )
        .unwrap_or_else(|error| panic!("render v1: {error}"));
    assert_eq!(first, "Agenda v1: history-v1");

    fs::write(&template_path, "Agenda v2: {{ task }}")
        .unwrap_or_else(|error| panic!("write v2 template: {error}"));

    let second = manager
        .render_target(
            &ManifestationTemplateTarget::DailyAgenda,
            json!({ "task": "history-v2" }),
        )
        .unwrap_or_else(|error| panic!("render v2 after hot reload: {error}"));
    assert_eq!(second, "Agenda v2: history-v2");
}

#[test]
fn manifestation_manager_rejects_invalid_template_glob() {
    let error = ManifestationManager::new(&["["])
        .err()
        .unwrap_or_else(|| panic!("invalid glob should fail"));
    assert!(error.to_string().contains("Invalid template glob"));
}
