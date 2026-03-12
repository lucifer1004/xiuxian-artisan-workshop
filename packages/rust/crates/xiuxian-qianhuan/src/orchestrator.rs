//! Multi-layer orchestrator for xiuxian-qianhuan prompt assembly.

use std::sync::Arc;

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
}

/// Assembles layered prompt snapshots with optional narrative transmutation.
pub struct ThousandFacesOrchestrator {
    genesis_rules: String,
    transmuter: Option<Arc<dyn ToneTransmuter>>,
}

impl ThousandFacesOrchestrator {
    /// Creates a new orchestrator with fixed genesis rules and optional transmuter.
    pub fn new(genesis_rules: String, transmuter: Option<Arc<dyn ToneTransmuter>>) -> Self {
        Self {
            genesis_rules,
            transmuter,
        }
    }

    /// Assembles the final XML system prompt snapshot asynchronously.
    ///
    /// Narrative blocks are passed through the configured transmuter when present.
    pub async fn assemble_snapshot(
        &self,
        persona: &PersonaProfile,
        narrative_blocks: Vec<String>,
        history: &str,
    ) -> Result<String, InjectionError> {
        let mut full_prompt = String::with_capacity(4096);

        // L0: Genesis (Immutable Core)
        full_prompt.push_str(&format!(
            "<genesis_rules>\n{}\n</genesis_rules>\n",
            self.genesis_rules
        ));

        // L1: Persona Steering (Automatic Translation from YAML logic to XML protocol)
        full_prompt.push_str("<persona_steering>\n");
        full_prompt.push_str(&format!("  <tone>{}</tone>\n", persona.voice_tone));
        full_prompt.push_str(&format!(
            "  <thought_pattern>{}</thought_pattern>\n",
            persona.cot_template
        ));
        full_prompt.push_str("  <anchors>");
        full_prompt.push_str(&persona.style_anchors.join(", "));
        full_prompt.push_str("</anchors>\n");
        full_prompt.push_str("</persona_steering>\n");

        // L2: Narrative Context (With optional Transmutation)
        full_prompt.push_str("<narrative_context>\n");
        if let Some(ref transmuter) = self.transmuter {
            for block in narrative_blocks {
                let shifted = transmuter.transmute(&block, persona).await?;
                full_prompt.push_str(&format!("  <entry>{}</entry>\n", shifted));
            }
        } else {
            for block in narrative_blocks {
                full_prompt.push_str(&format!("  <entry>{}</entry>\n", block));
            }
        }
        full_prompt.push_str("</narrative_context>\n");

        // L3: Working Window (Recency)
        full_prompt.push_str(&format!(
            "<working_history>\n{}\n</working_history>\n",
            history
        ));

        // Final Root Wrap
        let final_xml = format!(
            "<{}>\n{}\n</{}>",
            SYSTEM_PROMPT_INJECTION_TAG, full_prompt, SYSTEM_PROMPT_INJECTION_TAG
        );

        // 2026 Integrity Check: Validate XML balance before returning
        self.validate_xml(&final_xml)?;

        Ok(final_xml)
    }

    fn validate_xml(&self, xml: &str) -> Result<(), InjectionError> {
        let mut stack = Vec::new();
        let mut i = 0;
        let bytes = xml.as_bytes();

        while i < bytes.len() {
            if bytes[i] == b'<' {
                let start = i + 1;
                let mut j = start;
                while j < bytes.len() && bytes[j] != b'>' {
                    j += 1;
                }
                if j == bytes.len() {
                    return Err(InjectionError::XmlValidationError(
                        "Unclosed tag".to_string(),
                    ));
                }

                let tag_content = &xml[start..j];
                if tag_content.starts_with('/') {
                    // Closing tag
                    let tag_name = &tag_content[1..];
                    match stack.pop() {
                        Some(open_tag) if open_tag == tag_name => {}
                        Some(open_tag) => {
                            return Err(InjectionError::XmlValidationError(format!(
                                "Mismatched tag: expected </{}>, found </{}>",
                                open_tag, tag_name
                            )));
                        }
                        None => {
                            return Err(InjectionError::XmlValidationError(format!(
                                "Unexpected closing tag: </{}>",
                                tag_name
                            )));
                        }
                    }
                } else if !tag_content.ends_with('/') {
                    // Opening tag (ignoring self-closing tags like <br/>)
                    // Split by space to ignore attributes if any
                    let tag_name = tag_content.split_whitespace().next().unwrap_or("");
                    if !tag_name.is_empty() {
                        stack.push(tag_name);
                    }
                }
                i = j + 1;
            } else {
                i += 1;
            }
        }

        if let Some(open_tag) = stack.pop() {
            return Err(InjectionError::XmlValidationError(format!(
                "Unclosed tag at end of input: <{}>",
                open_tag
            )));
        }

        Ok(())
    }
}
