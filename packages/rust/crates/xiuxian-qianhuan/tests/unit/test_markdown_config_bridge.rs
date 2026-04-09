//! End-to-end runtime bridge validation for memory-loaded persona and templates.

use anyhow::Result;
use serde_json::json;
use std::collections::HashMap;
use xiuxian_qianhuan::{
    ManifestationManager, ManifestationRenderRequest, ManifestationRuntimeContext,
    ManifestationTemplateTarget, MemoryPersonaRecord, MemoryTemplateRecord, PersonaRegistry,
};

#[test]
fn runtime_bridge_loads_persona_and_template_from_memory_records() -> Result<()> {
    let mut registry = PersonaRegistry::new();
    let loaded_personas = registry.load_from_memory_records([MemoryPersonaRecord::new(
        "agenda_steward",
        r#"
name = "Agenda Steward"
voice_tone = "Structured and practical."
background = "Keeps plans concrete."
guidelines = ["Stay actionable"]
style_anchors = ["agenda", "clarity"]
cot_template = "Observe -> draft -> validate"
forbidden_words = ["impossible"]
"#,
    )])?;
    assert_eq!(loaded_personas, 1);
    assert!(registry.get("agenda_steward").is_some());

    let manager =
        ManifestationManager::new_with_embedded_templates(&[], &[("bootstrap.md", "bootstrap")])?;
    let loaded_templates = manager.load_templates_from_memory([MemoryTemplateRecord::new(
        "draft_agenda.j2",
        Some("daily_agenda.md".to_string()),
        r"Agenda owner: {{ user }}
Persona: {{ qianhuan.persona_id }}
Task: {{ task }}",
    )])?;
    assert_eq!(loaded_templates, 2);

    let request = ManifestationRenderRequest {
        target: ManifestationTemplateTarget::DailyAgenda,
        data: json!({
            "user": "Taogege",
            "task": "Validate markdown-config bridge",
        }),
        runtime: ManifestationRuntimeContext {
            state_context: None,
            persona_id: Some("agenda_steward".to_string()),
            domain: Some("zhixing".to_string()),
            extra: HashMap::new(),
        },
    };
    let rendered = manager.render_request(&request)?;

    assert!(rendered.contains("Agenda owner: Taogege"));
    assert!(rendered.contains("Persona: agenda_steward"));
    assert!(rendered.contains("Task: Validate markdown-config bridge"));
    Ok(())
}
