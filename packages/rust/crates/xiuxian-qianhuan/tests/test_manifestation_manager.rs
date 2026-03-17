//! Integration tests for manifestation manager template rendering.

use serde_json::json;
use std::fs;
use tempfile::tempdir;
use xiuxian_qianhuan::{
    ManifestationInterface, ManifestationManager, ManifestationRenderRequest,
    ManifestationRuntimeContext, ManifestationTemplateTarget,
};

#[test]
fn manifestation_manager_renders_template() {
    let dir = tempdir().expect("create temp dir");
    let template_path = dir.path().join("test.md.j2");
    fs::write(&template_path, "Hello {{ name }}!").expect("write template");

    let glob = format!("{}/*.j2", dir.path().display());
    let manager =
        ManifestationManager::new(&[glob.as_str()]).expect("create manifestation manager");

    let rendered = manager
        .render_template("test.md.j2", json!({"name": "Daoist"}))
        .expect("render template");
    assert_eq!(rendered, "Hello Daoist!");
}

#[test]
fn manifestation_manager_render_request_injects_runtime_context() {
    let dir = tempdir().expect("create temp dir");
    fs::write(
        dir.path().join("system_prompt_v2.xml"),
        "<root>{{ qianhuan.persona_id }}|{{ qianhuan.state_context }}|{{ qianhuan.injected_context }}</root>",
    )
    .expect("write xml template");

    let glob = format!("{}/*", dir.path().display());
    let manager =
        ManifestationManager::new(&[glob.as_str()]).expect("create manifestation manager");

    let request = ManifestationRenderRequest {
        target: ManifestationTemplateTarget::SystemPromptV2Xml,
        data: json!({}),
        runtime: ManifestationRuntimeContext {
            state_context: Some("STALE_TASKS".to_string()),
            persona_id: Some("artisan-engineer".to_string()),
            domain: Some("zhixing".to_string()),
            extra: Default::default(),
        },
    };

    let rendered = manager.render_request(&request).expect("render request");
    assert!(rendered.contains("artisan-engineer"));
    assert!(rendered.contains("STALE_TASKS"));
    assert!(rendered.contains("Cognitive Interface Warning"));
}

#[test]
fn manifestation_manager_supports_multiple_template_targets() {
    let dir = tempdir().expect("create temp dir");
    fs::write(dir.path().join("daily_agenda.md"), "Agenda: {{ title }}").expect("write agenda");
    fs::write(
        dir.path().join("system_prompt_v2.xml"),
        "<prompt>{{ title }}</prompt>",
    )
    .expect("write xml");

    let glob = format!("{}/*", dir.path().display());
    let manager =
        ManifestationManager::new(&[glob.as_str()]).expect("create manifestation manager");

    let agenda = manager
        .render_target(
            &ManifestationTemplateTarget::DailyAgenda,
            json!({"title": "Morning Cultivation"}),
        )
        .expect("render agenda");
    assert_eq!(agenda, "Agenda: Morning Cultivation");

    let xml = manager
        .render_target(
            &ManifestationTemplateTarget::SystemPromptV2Xml,
            json!({"title": "Runtime Persona"}),
        )
        .expect("render system prompt");
    assert_eq!(xml, "<prompt>Runtime Persona</prompt>");
}

#[test]
fn manifestation_manager_hot_reloads_template_without_restart() {
    let dir = tempdir().expect("create temp dir");
    let template_path = dir.path().join("daily_agenda.md");
    fs::write(&template_path, "Agenda v1: {{ title }}").expect("write v1 template");

    let glob = format!("{}/*", dir.path().display());
    let manager =
        ManifestationManager::new(&[glob.as_str()]).expect("create manifestation manager");

    let first = manager
        .render_target(
            &ManifestationTemplateTarget::DailyAgenda,
            json!({"title": "Morning"}),
        )
        .expect("render v1");
    assert_eq!(first, "Agenda v1: Morning");

    fs::write(&template_path, "Agenda v2: {{ title }}").expect("write v2 template");

    let second = manager
        .render_target(
            &ManifestationTemplateTarget::DailyAgenda,
            json!({"title": "Morning"}),
        )
        .expect("render v2 after hot reload");
    assert_eq!(second, "Agenda v2: Morning");
}
