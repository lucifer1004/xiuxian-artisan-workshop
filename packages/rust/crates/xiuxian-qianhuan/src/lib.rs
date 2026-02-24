//! System prompt injection window based on XML Q&A blocks.
//!
//! Contract:
//! - Root tag: `<system_prompt_injection>`
//! - Entry tag: `<qa><q>...</q><a>...</a><source>...</source></qa>`
//! - `<source>` is optional.

/// Synapse-Audit calibration primitives for adversarial alignment checks.
pub mod calibration;
mod config;
mod contracts;
mod entry;
mod error;
/// Orchestration layer for multi-layer prompt assembly.
pub mod orchestrator;
/// Persona model and registry for role-mix style injection.
pub mod persona;
/// Python bindings for the thin orchestration/persona API surface.
pub mod python_module;
/// Tone transmutation traits and implementations.
pub mod transmuter;
mod window;
mod xml;

pub use config::InjectionWindowConfig;
pub use contracts::{
    InjectionMode, InjectionOrderStrategy, InjectionPolicy, InjectionSnapshot, PromptContextBlock,
    PromptContextCategory, PromptContextSource, RoleMixProfile, RoleMixRole,
};
pub use entry::QaEntry;
pub use error::InjectionError;
pub use orchestrator::{InjectionLayer, ThousandFacesOrchestrator};
pub use persona::{PersonaProfile, PersonaRegistry};
pub use transmuter::{MockTransmuter, ToneTransmuter};
pub use window::SystemPromptInjectionWindow;
pub use xml::SYSTEM_PROMPT_INJECTION_TAG;
