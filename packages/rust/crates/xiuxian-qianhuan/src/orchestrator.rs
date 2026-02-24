//! Multi-layer orchestrator for xiuxian-qianhuan prompt assembly.

use std::fmt::Write as _;
use std::sync::Arc;

use crate::calibration::AdversarialOrchestrator;
use crate::error::InjectionError;
use crate::persona::PersonaProfile;
use crate::transmuter::ToneTransmuter;
use crate::xml::SYSTEM_PROMPT_INJECTION_TAG;

/// Logical layers used to compose an injection snapshot.
pub enum InjectionLayer {
    /// L0: immutable safety and governance rules.
    Genesis,
    /// L1: persona tone and reasoning style steering.
    Persona,
    /// L2: transformed narrative/knowledge blocks.
    Narrative,
    /// L3: recency/working-memory context.
    Working,
    /// 2026 Extension: Calibration feedback from Skeptic.
    Calibration,
}

/// Assembles layered prompt snapshots with optional narrative transmutation.
pub struct ThousandFacesOrchestrator {
    genesis_rules: String,
    transmuter: Option<Arc<dyn ToneTransmuter>>,
    /// Optional adversarial calibrator for post-assembly alignment loops.
    pub calibrator: Option<Arc<AdversarialOrchestrator>>,
}

impl ThousandFacesOrchestrator {
    /// Creates a new orchestrator with fixed genesis rules and optional transmuter.
    #[must_use]
    pub fn new(genesis_rules: String, transmuter: Option<Arc<dyn ToneTransmuter>>) -> Self {
        Self {
            genesis_rules,
            transmuter,
            calibrator: None,
        }
    }

    /// Assembles the final XML system prompt snapshot asynchronously.
    ///
    /// Narrative blocks are passed through the configured transmuter when present.
    ///
    /// # Errors
    ///
    /// Returns [`InjectionError`] when context completeness is below threshold,
    /// tone transmutation fails, or generated XML is invalid.
    pub async fn assemble_snapshot(
        &self,
        persona: &PersonaProfile,
        narrative_blocks: Vec<String>,
        history: &str,
    ) -> Result<String, InjectionError> {
        // 2026 CCS Gating: Evaluate if facts support the persona (Ref: Agent-G)
        let (ccs, missing_anchors) = Self::calculate_ccs_with_missing(persona, &narrative_blocks);
        if ccs < 0.65 {
            return Err(InjectionError::ContextInsufficient {
                ccs,
                missing_info: missing_anchors.join(", "),
            });
        }

        let mut full_prompt = String::with_capacity(4096);
        let mut pushf = |args: std::fmt::Arguments<'_>| {
            full_prompt
                .write_fmt(args)
                .map_err(|_| InjectionError::XmlValidationError("failed to compose prompt".into()))
        };

        // L0: Genesis (Immutable Core)
        pushf(format_args!(
            "<genesis_rules>\n{}\n</genesis_rules>\n",
            self.genesis_rules
        ))?;

        // L1: Persona Steering (Automatic Translation from YAML logic to XML protocol)
        pushf(format_args!("<persona_steering>\n"))?;
        pushf(format_args!("  <tone>{}</tone>\n", persona.voice_tone))?;
        pushf(format_args!(
            "  <thought_pattern>{}</thought_pattern>\n",
            persona.cot_template
        ))?;
        pushf(format_args!("  <anchors>"))?;
        pushf(format_args!("{}", persona.style_anchors.join(", ")))?;
        pushf(format_args!("</anchors>\n"))?;
        pushf(format_args!("</persona_steering>\n"))?;

        // L2: Narrative Context (With optional Transmutation)
        pushf(format_args!("<narrative_context>\n"))?;
        if let Some(ref transmuter) = self.transmuter {
            for block in narrative_blocks {
                let shifted = transmuter.transmute(&block, persona).await?;
                pushf(format_args!("  <entry>{shifted}</entry>\n"))?;
            }
        } else {
            for block in narrative_blocks {
                pushf(format_args!("  <entry>{block}</entry>\n"))?;
            }
        }
        pushf(format_args!("</narrative_context>\n"))?;

        // L3: Working Window (Recency)
        pushf(format_args!(
            "<working_history>\n{history}\n</working_history>\n"
        ))?;

        // Final Root Wrap
        let final_xml = format!(
            "<{SYSTEM_PROMPT_INJECTION_TAG}>\n{full_prompt}\n</{SYSTEM_PROMPT_INJECTION_TAG}>"
        );

        // 2026 Integrity Check: Validate XML balance before returning
        Self::validate_xml(&final_xml)?;

        Ok(final_xml)
    }

    fn validate_xml(xml: &str) -> Result<(), InjectionError> {
        use quick_xml::Reader;
        use quick_xml::events::Event;

        let mut reader = Reader::from_str(xml);
        let mut stack = Vec::new();

        loop {
            match reader.read_event() {
                Ok(Event::Start(e)) => {
                    let name = String::from_utf8_lossy(e.name().as_ref()).to_string();
                    stack.push(name);
                }
                Ok(Event::End(e)) => {
                    let name = String::from_utf8_lossy(e.name().as_ref()).to_string();
                    if let Some(open_tag) = stack.pop() {
                        if open_tag != name {
                            return Err(InjectionError::XmlValidationError(format!(
                                "Mismatched tag: expected </{open_tag}>, found </{name}>"
                            )));
                        }
                    } else {
                        return Err(InjectionError::XmlValidationError(format!(
                            "Unexpected closing tag: </{name}>"
                        )));
                    }
                }
                Ok(Event::Eof) => break,
                Err(e) => {
                    return Err(InjectionError::XmlValidationError(format!(
                        "Malformed XML structure: {e}"
                    )));
                }
                _ => {}
            }
        }

        if let Some(open_tag) = stack.pop() {
            return Err(InjectionError::XmlValidationError(format!(
                "Unclosed tag: <{open_tag}>"
            )));
        }

        Ok(())
    }

    /// Calculates Context Completeness Score (CCS) and identifies missing anchors.
    fn calculate_ccs_with_missing(
        persona: &PersonaProfile,
        narrative: &[String],
    ) -> (f64, Vec<String>) {
        if persona.style_anchors.is_empty() {
            return (1.0, Vec::new());
        }
        if narrative.is_empty() {
            return (0.0, persona.style_anchors.clone());
        }

        let mut missing = Vec::new();
        let mut matches: usize = 0;
        for anchor in &persona.style_anchors {
            let anchor_lower = anchor.to_lowercase();
            let mut found = false;
            for block in narrative {
                if block.to_lowercase().contains(&anchor_lower) {
                    found = true;
                    break;
                }
            }
            if found {
                matches += 1;
            } else {
                missing.push(anchor.clone());
            }
        }

        let total = u32::try_from(persona.style_anchors.len()).unwrap_or(u32::MAX);
        let matched_count = u32::try_from(matches).unwrap_or(total);
        let score = if total == 0 {
            1.0
        } else {
            (f64::from(matched_count) / f64::from(total)).clamp(0.0, 1.0)
        };
        (score, missing)
    }
}
