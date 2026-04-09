//! Integration tests for manifestation manager template rendering.

use serde_json::json;
use std::fs;
use tempfile::tempdir;
use xiuxian_qianhuan::{
    ManifestationInterface, ManifestationManager, ManifestationRenderRequest,
    ManifestationRuntimeContext, ManifestationTemplateTarget,
};

#[test]
fn manifestation_manager_renders_template() -> Result<(), Box<dyn std::error::Error>> {
    let dir = tempdir()?;
    let template_path = dir.path().join("test.md.j2");
    fs::write(&template_path, "Hello {{ name }}!")?;

    let glob = format!("{}/*.j2", dir.path().display());
    let manager = ManifestationManager::new(&[glob.as_str()])?;

    let rendered = manager.render_template("test.md.j2", json!({"name": "Daoist"}))?;
    assert_eq!(rendered, "Hello Daoist!");
    Ok(())
}

#[test]
fn manifestation_manager_render_request_injects_runtime_context()
-> Result<(), Box<dyn std::error::Error>> {
    let dir = tempdir()?;
    fs::write(
        dir.path().join("system_prompt_v2.xml"),
        "<root>{{ qianhuan.persona_id }}|{{ qianhuan.state_context }}|{{ qianhuan.injected_context }}</root>",
    )?;

    let glob = format!("{}/*", dir.path().display());
    let manager = ManifestationManager::new(&[glob.as_str()])?;

    let request = ManifestationRenderRequest {
        target: ManifestationTemplateTarget::SystemPromptV2Xml,
        data: json!({}),
        runtime: ManifestationRuntimeContext {
            state_context: Some("STALE_TASKS".to_string()),
            persona_id: Some("artisan-engineer".to_string()),
            domain: Some("zhixing".to_string()),
            extra: std::collections::HashMap::default(),
        },
    };

    let rendered = manager.render_request(&request)?;
    assert!(rendered.contains("artisan-engineer"));
    assert!(rendered.contains("STALE_TASKS"));
    assert!(rendered.contains("Cognitive Interface Warning"));
    Ok(())
}

#[test]
fn manifestation_manager_supports_multiple_template_targets()
-> Result<(), Box<dyn std::error::Error>> {
    let dir = tempdir()?;
    fs::write(dir.path().join("daily_agenda.md"), "Agenda: {{ title }}")?;
    fs::write(
        dir.path().join("system_prompt_v2.xml"),
        "<prompt>{{ title }}</prompt>",
    )?;

    let glob = format!("{}/*", dir.path().display());
    let manager = ManifestationManager::new(&[glob.as_str()])?;

    let agenda = manager.render_target(
        &ManifestationTemplateTarget::DailyAgenda,
        json!({"title": "Morning Cultivation"}),
    )?;
    assert_eq!(agenda, "Agenda: Morning Cultivation");

    let xml = manager.render_target(
        &ManifestationTemplateTarget::SystemPromptV2Xml,
        json!({"title": "Runtime Persona"}),
    )?;
    assert_eq!(xml, "<prompt>Runtime Persona</prompt>");
    Ok(())
}

#[test]
fn manifestation_manager_hot_reloads_template_without_restart()
-> Result<(), Box<dyn std::error::Error>> {
    let dir = tempdir()?;
    let template_path = dir.path().join("daily_agenda.md");
    fs::write(&template_path, "Agenda v1: {{ title }}")?;

    let glob = format!("{}/*", dir.path().display());
    let manager = ManifestationManager::new(&[glob.as_str()])?;

    let first = manager.render_target(
        &ManifestationTemplateTarget::DailyAgenda,
        json!({"title": "Morning"}),
    )?;
    assert_eq!(first, "Agenda v1: Morning");

    fs::write(&template_path, "Agenda v2: {{ title }}")?;

    let second = manager.render_target(
        &ManifestationTemplateTarget::DailyAgenda,
        json!({"title": "Morning"}),
    )?;
    assert_eq!(second, "Agenda v2: Morning");
    Ok(())
}
