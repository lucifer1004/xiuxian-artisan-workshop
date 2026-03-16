//! Lightweight content washing for Spider ingress.

use thiserror::Error;

use crate::xml_lite::{extract_tag_f32, extract_tag_value};

/// Structural validation failures detected during ingress washing.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum ZhenfaTransmuterError {
    /// Input contains null bytes and is rejected before assimilation.
    #[error("input contains null bytes")]
    NullByteDetected,
    /// Closing tag did not match the latest opening tag.
    #[error("mismatched XML-Lite tag: expected </{expected}>, found </{found}>")]
    MismatchedClosingTag {
        /// The opening tag waiting to be closed.
        expected: String,
        /// The closing tag found in the payload.
        found: String,
    },
    /// Closing tag appeared without a corresponding opening tag.
    #[error("unexpected XML-Lite closing tag </{found}>")]
    UnexpectedClosingTag {
        /// The closing tag that could not be matched.
        found: String,
    },
    /// Input ended while some opening tags were still unclosed.
    #[error("unclosed XML-Lite tag <{tag}>")]
    UnclosedTag {
        /// The opening tag that remained on stack.
        tag: String,
    },
}

impl ZhenfaTransmuterError {
    /// Returns one LLM-safe semantic summary of the error.
    #[must_use]
    pub fn llm_safe_message(&self) -> &'static str {
        match self {
            Self::NullByteDetected => {
                "content contains unsupported control characters; clean the payload and retry"
            }
            Self::MismatchedClosingTag { .. }
            | Self::UnexpectedClosingTag { .. }
            | Self::UnclosedTag { .. } => {
                "content has malformed XML-Lite structure; ensure all tags are balanced"
            }
        }
    }
}

/// Failures for content washing plus structural validation.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum ZhenfaResolveAndWashError {
    /// The supplied content was empty after trimming.
    #[error("semantic resource URI `{uri}` could not be resolved")]
    ResourceNotFound {
        /// Canonical resource URI.
        uri: String,
    },
    /// Structural validation failed after content washing.
    #[error(transparent)]
    Transmuter(#[from] ZhenfaTransmuterError),
}

/// Shared transmutation routines for XML-lite validation and normalization.
#[derive(Debug, Default, Clone, Copy)]
pub struct ZhenfaTransmuter;

impl ZhenfaTransmuter {
    /// Validate XML-lite structure in-place.
    ///
    /// # Errors
    ///
    /// Returns [`ZhenfaTransmuterError`] when the payload is malformed.
    pub fn validate_structure(content: &str) -> Result<(), ZhenfaTransmuterError> {
        validate_structure(content)
    }

    /// Normalize payload for LLM consumption.
    #[must_use]
    pub fn refine_for_llm(content: &str) -> String {
        refine_for_llm(content)
    }

    /// Validate XML-lite structure and return the refined payload.
    ///
    /// # Errors
    ///
    /// Returns [`ZhenfaTransmuterError`] when the payload is malformed.
    pub fn validate_and_refine(content: &str) -> Result<String, ZhenfaTransmuterError> {
        let refined = refine_for_llm(content);
        validate_structure(&refined)?;
        Ok(refined)
    }

    /// Resolve one already-loaded payload and apply lightweight washing.
    ///
    /// # Errors
    ///
    /// Returns [`ZhenfaResolveAndWashError::ResourceNotFound`] when the resolver yields no text.
    /// Returns [`ZhenfaResolveAndWashError::Transmuter`] when XML-Lite validation fails.
    pub fn resolve_and_wash<F>(uri: &str, resolver: F) -> Result<String, ZhenfaResolveAndWashError>
    where
        F: FnOnce(&str) -> Option<String>,
    {
        let canonical_uri = uri.trim();
        let raw_content =
            resolver(canonical_uri).ok_or_else(|| ZhenfaResolveAndWashError::ResourceNotFound {
                uri: canonical_uri.to_string(),
            })?;
        let refined = refine_for_llm(raw_content.as_str());
        if should_validate_xml_lite(canonical_uri) {
            validate_structure(refined.as_str())?;
        }
        Ok(refined)
    }

    /// Check for semantic integrity of reference anchors.
    #[must_use]
    pub fn check_semantic_integrity(content: &str) -> bool {
        check_semantic_integrity(content)
    }

    /// Extract text content of the first `<tag>...</tag>` block.
    #[must_use]
    pub fn get_tag_value<'a>(text: &'a str, tag: &str) -> Option<&'a str> {
        extract_tag_value(text, tag)
    }

    /// Parse the first `<tag>...</tag>` block as `f32`.
    #[must_use]
    pub fn get_tag_f32(text: &str, tag: &str) -> Option<f32> {
        extract_tag_f32(text, tag)
    }
}

/// Check for semantic integrity of reference anchors.
#[must_use]
pub fn check_semantic_integrity(content: &str) -> bool {
    let mut cursor = 0usize;
    while let Some(offset) = content[cursor..].find("[[references/") {
        let link_start = cursor + offset + 2;
        let rest = &content[link_start..];
        let Some(end) = rest.find("]]") else {
            return false;
        };
        let link = &rest[..end];
        if !link.contains('#') {
            return false;
        }
        cursor = link_start + end + 2;
    }
    true
}

/// Normalize payload for LLM consumption.
#[must_use]
pub fn refine_for_llm(content: &str) -> String {
    let normalized_line_endings = content.replace("\r\n", "\n").replace('\r', "\n");
    let sanitized = normalized_line_endings.replace('\0', "");

    let mut refined = String::with_capacity(sanitized.len());
    let mut blank_run = 0usize;
    for line in sanitized.lines() {
        let trimmed_end = line.trim_end();
        if trimmed_end.is_empty() {
            blank_run += 1;
            if blank_run > 2 {
                continue;
            }
        } else {
            blank_run = 0;
        }

        if !refined.is_empty() {
            refined.push('\n');
        }
        refined.push_str(trimmed_end);
    }

    refined.trim().to_string()
}

/// Validate XML-lite structure.
///
/// # Errors
///
/// Returns [`ZhenfaTransmuterError`] when the payload is malformed.
pub fn validate_structure(content: &str) -> Result<(), ZhenfaTransmuterError> {
    if content.contains('\0') {
        return Err(ZhenfaTransmuterError::NullByteDetected);
    }

    let bytes = content.as_bytes();
    let mut cursor = 0usize;
    let mut stack: Vec<String> = Vec::new();

    while cursor < bytes.len() {
        if bytes[cursor] != b'<' {
            cursor += 1;
            continue;
        }

        if cursor + 1 >= bytes.len() {
            break;
        }

        if bytes[cursor + 1] == b'!' {
            if content[cursor..].starts_with("<!--") {
                if let Some(offset) = content[cursor + 4..].find("-->") {
                    cursor = cursor + 4 + offset + 3;
                    continue;
                }
                return Err(ZhenfaTransmuterError::UnclosedTag {
                    tag: "!--".to_string(),
                });
            }
            cursor += 1;
            continue;
        }

        if bytes[cursor + 1] == b'?' {
            if let Some(offset) = content[cursor + 2..].find("?>") {
                cursor = cursor + 2 + offset + 2;
                continue;
            }
            break;
        }

        let closing = bytes[cursor + 1] == b'/';
        let tag_start = if closing { cursor + 2 } else { cursor + 1 };
        if tag_start >= bytes.len() {
            break;
        }
        if !is_tag_name_start(bytes[tag_start]) {
            cursor += 1;
            continue;
        }

        let mut tag_end = tag_start + 1;
        while tag_end < bytes.len() && is_tag_name_char(bytes[tag_end]) {
            tag_end += 1;
        }
        let tag_name = &content[tag_start..tag_end];

        let mut angle_close = tag_end;
        while angle_close < bytes.len() && bytes[angle_close] != b'>' {
            angle_close += 1;
        }
        if angle_close >= bytes.len() {
            return Err(ZhenfaTransmuterError::UnclosedTag {
                tag: tag_name.to_string(),
            });
        }

        let self_closing = !closing && angle_close > cursor && bytes[angle_close - 1] == b'/';
        if closing {
            match stack.pop() {
                Some(expected) if expected == tag_name => {}
                Some(expected) => {
                    return Err(ZhenfaTransmuterError::MismatchedClosingTag {
                        expected,
                        found: tag_name.to_string(),
                    });
                }
                None => {
                    return Err(ZhenfaTransmuterError::UnexpectedClosingTag {
                        found: tag_name.to_string(),
                    });
                }
            }
        } else if !self_closing {
            stack.push(tag_name.to_string());
        }

        cursor = angle_close + 1;
    }

    if let Some(tag) = stack.pop() {
        return Err(ZhenfaTransmuterError::UnclosedTag { tag });
    }
    Ok(())
}

fn is_tag_name_start(byte: u8) -> bool {
    byte.is_ascii_alphabetic() || byte == b'_'
}

fn is_tag_name_char(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-' | b':')
}

fn should_validate_xml_lite(uri: &str) -> bool {
    let extension = uri
        .rsplit('.')
        .next()
        .map(str::trim)
        .map(str::to_ascii_lowercase);
    matches!(extension.as_deref(), Some("xml" | "xml-lite" | "xlite"))
}
