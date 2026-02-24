//! Tone transmutation abstraction for persona-aware narrative rewriting.

use crate::error::InjectionError;
use crate::persona::PersonaProfile;
use async_trait::async_trait;

/// Trait for converting raw facts into persona-aligned narrative text.
#[async_trait]
pub trait ToneTransmuter: Send + Sync {
    /// Transmutes a technical fact into a persona-aligned narrative.
    async fn transmute(
        &self,
        raw_fact: &str,
        persona: &PersonaProfile,
    ) -> Result<String, InjectionError>;
}

/// A simple implementation for local verification and CI.
pub struct MockTransmuter;

#[async_trait]
impl ToneTransmuter for MockTransmuter {
    async fn transmute(
        &self,
        raw_fact: &str,
        persona: &PersonaProfile,
    ) -> Result<String, InjectionError> {
        // Simulation logic: Map keywords based on persona ID
        let shifted = if persona.id.contains("cultivator") {
            format!("The Dao reveals: {raw_fact}. (Refining through the zenith of computation)")
        } else if persona.id.contains("artisan") {
            format!("Artisan Report: {raw_fact}. (Verified via millimeter-level audit trail)")
        } else {
            format!(
                "[{name}] {tone}: {raw_fact}",
                name = persona.name,
                tone = persona.voice_tone
            )
        };

        Ok(shifted)
    }
}
