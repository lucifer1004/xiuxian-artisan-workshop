//! Persona model and registry for xiuxian-qianhuan.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Profile defining an AI persona's voice, constraints and reasoning style.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersonaProfile {
    /// Unique identifier for the persona.
    pub id: String,
    /// Friendly display name.
    pub name: String,
    /// Detailed description of the voice and tone.
    pub voice_tone: String,
    /// Keywords or anchors that must be present in the grounding context.
    pub style_anchors: Vec<String>,
    /// Template used for Chain-of-Thought reasoning.
    pub cot_template: String,
    /// List of phrases the persona is forbidden to use.
    pub forbidden_words: Vec<String>,
    /// Optional metadata for extended persona traits.
    #[serde(default)]
    pub metadata: HashMap<String, String>,
}

/// Registry managing the collection of available personas.
pub struct PersonaRegistry {
    personas: HashMap<String, PersonaProfile>,
}

impl PersonaRegistry {
    /// Creates a new registry with built-in personas loaded from TOML.
    #[must_use]
    pub fn with_builtins() -> Self {
        let mut personas = HashMap::new();

        let artisan_toml = include_str!("../resources/personas/artisan.toml");
        let cultivator_toml = include_str!("../resources/personas/cultivator.toml");

        if let Ok(p) = toml::from_str::<PersonaProfile>(artisan_toml) {
            personas.insert(p.id.clone(), p);
        }
        if let Ok(p) = toml::from_str::<PersonaProfile>(cultivator_toml) {
            personas.insert(p.id.clone(), p);
        }

        Self { personas }
    }

    /// Fetches a persona profile by its unique ID.
    #[must_use]
    pub fn get(&self, id: &str) -> Option<&PersonaProfile> {
        self.personas.get(id)
    }

    /// Registers a custom persona into the registry.
    pub fn register(&mut self, profile: PersonaProfile) {
        self.personas.insert(profile.id.clone(), profile);
    }
}
