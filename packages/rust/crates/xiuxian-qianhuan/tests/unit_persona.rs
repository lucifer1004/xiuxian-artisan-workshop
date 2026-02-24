//! Unit tests for persona registry and profile operations.

use std::collections::HashMap;
use xiuxian_qianhuan::{PersonaProfile, PersonaRegistry};

#[test]
fn test_builtin_loading() {
    let registry = PersonaRegistry::with_builtins();
    let Some(artisan) = registry.get("artisan-engineer") else {
        panic!("Artisan should exist");
    };
    assert_eq!(artisan.name, "Artisan Engineer");
    assert!(artisan.style_anchors.contains(&"audit trail".to_string()));

    let Some(cultivator) = registry.get("cyber-cultivator") else {
        panic!("Cultivator should exist");
    };
    assert_eq!(cultivator.name, "Cyber-Cultivator");
}

#[test]
fn test_custom_registration() {
    let mut registry = PersonaRegistry::with_builtins();
    let profile = PersonaProfile {
        id: "test".to_string(),
        name: "Test".to_string(),
        voice_tone: "Test".to_string(),
        style_anchors: vec![],
        cot_template: "Test".to_string(),
        forbidden_words: vec![],
        metadata: HashMap::new(),
    };
    registry.register(profile);
    assert!(registry.get("test").is_some());
}
