//! Multi-layer orchestrator for xiuxian-qianhuan prompt assembly.

use std::fmt::Write as _;
use std::sync::Arc;

use crate::error::{InjectionError, Result};
use crate::persona::PersonaProfile;
use crate::transmuter::ToneTransmuter;
use crate::xml::SYSTEM_PROMPT_INJECTION_TAG;

const MIN_CONTEXT_CONFIDENCE: f64 = 0.65;

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
    #[must_use]
    pub fn new(genesis_rules: String, transmuter: Option<Arc<dyn ToneTransmuter>>) -> Self {
        Self {
            genesis_rules,
            transmuter,
        }
    }

    /// Assembles the final XML system prompt snapshot asynchronously.
    ///
    /// Narrative blocks are passed through the configured transmuter when present.
    ///
    /// # Errors
    ///
    /// Returns an error when narrative transmutation fails, XML formatting cannot
    /// be written into the output buffer, or the final XML payload is unbalanced.
    pub async fn assemble_snapshot(
        &self,
        persona: &PersonaProfile,
        narrative_blocks: Vec<String>,
        history: &str,
    ) -> Result<String> {
        Self::enforce_context_confidence(persona, &narrative_blocks)?;

        let mut full_prompt = String::with_capacity(4096);
        let genesis_rules = escape_xml_text(self.genesis_rules.as_str());

        // L0: Genesis (Immutable Core)
        write_xml(
            &mut full_prompt,
            format_args!("<genesis_rules>\n{genesis_rules}\n</genesis_rules>\n"),
        )?;

        // L1: Persona Steering (Automatic Translation from YAML logic to XML protocol)
        full_prompt.push_str("<persona_steering>\n");
        write_xml(
            &mut full_prompt,
            format_args!("  <tone>{}</tone>\n", escape_xml_text(&persona.voice_tone)),
        )?;
        write_xml(
            &mut full_prompt,
            format_args!(
                "  <thought_pattern>{}</thought_pattern>\n",
                escape_xml_text(&persona.cot_template)
            ),
        )?;
        if let Some(background) = persona.background.as_deref()
            && !background.trim().is_empty()
        {
            write_xml(
                &mut full_prompt,
                format_args!(
                    "  <background>{}</background>\n",
                    escape_xml_text(background)
                ),
            )?;
        }
        if !persona.guidelines.is_empty() {
            full_prompt.push_str("  <guidelines>\n");
            for guideline in &persona.guidelines {
                write_xml(
                    &mut full_prompt,
                    format_args!("    <rule>{}</rule>\n", escape_xml_text(guideline)),
                )?;
            }
            full_prompt.push_str("  </guidelines>\n");
        }
        full_prompt.push_str("  <anchors>");
        full_prompt.push_str(
            &persona
                .style_anchors
                .iter()
                .map(|anchor| escape_xml_text(anchor))
                .collect::<Vec<_>>()
                .join(", "),
        );
        full_prompt.push_str("</anchors>\n");
        if !persona.forbidden_words.is_empty() {
            full_prompt.push_str("  <forbidden_terms>\n");
            for term in &persona.forbidden_words {
                write_xml(
                    &mut full_prompt,
                    format_args!("    <term>{}</term>\n", escape_xml_text(term)),
                )?;
            }
            full_prompt.push_str("  </forbidden_terms>\n");
        }
        full_prompt.push_str("</persona_steering>\n");

        // L2: Narrative Context (With optional Transmutation)
        full_prompt.push_str("<narrative_context>\n");
        if let Some(ref transmuter) = self.transmuter {
            for block in narrative_blocks {
                let shifted = transmuter.transmute(&block, persona).await?;
                write_xml(
                    &mut full_prompt,
                    format_args!("  <entry>{}</entry>\n", escape_xml_text(&shifted)),
                )?;
            }
        } else {
            for block in narrative_blocks {
                write_xml(
                    &mut full_prompt,
                    format_args!("  <entry>{}</entry>\n", escape_xml_text(&block)),
                )?;
            }
        }
        full_prompt.push_str("</narrative_context>\n");

        // L3: Working Window (Recency)
        write_xml(
            &mut full_prompt,
            format_args!(
                "<working_history>\n{}\n</working_history>\n",
                escape_xml_text(history)
            ),
        )?;

        // Final Root Wrap
        let mut final_xml = String::with_capacity(full_prompt.len() + 64);
        write_xml(
            &mut final_xml,
            format_args!(
                "<{SYSTEM_PROMPT_INJECTION_TAG}>\n{full_prompt}\n</{SYSTEM_PROMPT_INJECTION_TAG}>"
            ),
        )?;

        // 2026 Integrity Check: Validate XML balance before returning
        Self::validate_xml(&final_xml)?;

        Ok(final_xml)
    }

    fn validate_xml(xml: &str) -> Result<()> {
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
                if let Some(tag_name) = tag_content.strip_prefix('/') {
                    // Closing tag
                    match stack.pop() {
                        Some(open_tag) if open_tag == tag_name => {}
                        Some(open_tag) => {
                            return Err(InjectionError::XmlValidationError(format!(
                                "Mismatched tag: expected </{open_tag}>, found </{tag_name}>"
                            )));
                        }
                        None => {
                            return Err(InjectionError::XmlValidationError(format!(
                                "Unexpected closing tag: </{tag_name}>"
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
                "Unclosed tag at end of input: <{open_tag}>"
            )));
        }

        Ok(())
    }

    fn enforce_context_confidence(
        persona: &PersonaProfile,
        narrative_blocks: &[String],
    ) -> Result<()> {
        if persona.style_anchors.is_empty() {
            return Ok(());
        }

        let evidence = narrative_blocks
            .iter()
            .map(|block| block.to_lowercase())
            .collect::<Vec<_>>();
        let mut missing = Vec::new();
        let mut matched = 0.0_f64;
        let mut total = 0.0_f64;

        for anchor in &persona.style_anchors {
            total += 1.0;
            let anchor_lower = anchor.to_lowercase();
            if evidence
                .iter()
                .any(|block| block.contains(anchor_lower.as_str()))
            {
                matched += 1.0;
            } else {
                missing.push(anchor.clone());
            }
        }

        let ccs = if total == 0.0 {
            1.0
        } else {
            (matched / total).clamp(0.0, 1.0)
        };
        if ccs < MIN_CONTEXT_CONFIDENCE {
            return Err(InjectionError::ContextInsufficient {
                ccs,
                missing_info: missing.join(", "),
            });
        }
        Ok(())
    }
}

fn write_xml(buffer: &mut String, args: std::fmt::Arguments<'_>) -> Result<()> {
    buffer.write_fmt(args).map_err(|error| {
        InjectionError::Internal(format!("failed to format XML snapshot: {error}"))
    })
}

fn escape_xml_text(value: &str) -> String {
    let mut escaped = String::with_capacity(value.len());
    for ch in value.chars() {
        match ch {
            '&' => escaped.push_str("&amp;"),
            '<' => escaped.push_str("&lt;"),
            '>' => escaped.push_str("&gt;"),
            '"' => escaped.push_str("&quot;"),
            '\'' => escaped.push_str("&apos;"),
            _ => escaped.push(ch),
        }
    }
    escaped
}
