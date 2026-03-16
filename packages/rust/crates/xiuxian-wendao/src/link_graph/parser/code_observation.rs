//! Code observation parsing for Blueprint v2.7 (Internal AST Integration).
//!
//! This module provides parsing support for the `:OBSERVE:` property drawer attribute,
//! enabling documentation to observe code patterns via `xiuxian-ast` structural queries.
//!
//! ## Format
//!
//! The `:OBSERVE:` attribute uses the following syntax:
//! ```markdown
//! :OBSERVE: lang:<language> "<sgrep-pattern>"
//! ```
//!
//! ## Example
//!
//! ```markdown
//! ## Storage Module
//! :OBSERVE: lang:rust "fn $NAME($$$ARGS) -> Result<$$$RET, $$$ERR>"
//! ```
//!
//! ## Resolution
//!
//! During indexing, the `LinkGraphIndex` resolves these patterns via `xiuxian-ast`,
//! binding document nodes to specific code byte-ranges.

use std::collections::HashMap;
use std::fmt;

use serde::{Deserialize, Serialize};

/// Parsed code observation entry from `:OBSERVE:` property drawer.
///
/// Represents a structural code pattern that this documentation section observes.
/// The pattern is validated by `xiuxian-ast` during the audit phase.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CodeObservation {
    /// Target language for the pattern (e.g., "rust", "python", "typescript").
    pub language: String,
    /// The sgrep/ast-grep pattern to match in source code.
    pub pattern: String,
    /// The original raw value from the property drawer (for diagnostics).
    pub raw_value: String,
    /// Line number within the document where this observation was declared.
    pub line_number: Option<usize>,
    /// Whether the pattern has been validated by xiuxian-ast.
    pub is_validated: bool,
    /// Validation error message if pattern validation failed.
    pub validation_error: Option<String>,
}

impl CodeObservation {
    /// Create a new code observation.
    #[must_use]
    pub fn new(language: String, pattern: String, raw_value: String) -> Self {
        Self {
            language,
            pattern,
            raw_value,
            line_number: None,
            is_validated: false,
            validation_error: None,
        }
    }

    /// Create a code observation with line number.
    #[must_use]
    pub fn with_line(mut self, line_number: usize) -> Self {
        self.line_number = Some(line_number);
        self
    }

    /// Mark this observation as validated.
    #[must_use]
    pub fn validated(mut self) -> Self {
        self.is_validated = true;
        self
    }

    /// Mark this observation as having a validation error.
    #[must_use]
    pub fn with_error(mut self, error: String) -> Self {
        self.validation_error = Some(error);
        self
    }

    /// Parse a `:OBSERVE:` value string into a `CodeObservation`.
    ///
    /// # Format
    ///
    /// `lang:<language> "<pattern>"`
    ///
    /// # Examples
    ///
    /// ```
    /// use xiuxian_wendao::link_graph::parser::code_observation::CodeObservation;
    ///
    /// let obs = CodeObservation::parse(r#"lang:rust "fn $NAME($$$ARGS) -> Result<$$$RET, $$$ERR>""#);
    /// assert!(obs.is_some());
    /// let obs = obs.unwrap();
    /// assert_eq!(obs.language, "rust");
    /// assert_eq!(obs.pattern, "fn $NAME($$$ARGS) -> Result<$$$RET, $$$ERR>");
    /// ```
    #[must_use]
    pub fn parse(value: &str) -> Option<Self> {
        let trimmed = value.trim();

        // Must start with "lang:"
        if !trimmed.starts_with("lang:") {
            return None;
        }

        // Find the space after "lang:<language>"
        let after_lang = &trimmed[5..]; // Skip "lang:"
        let space_pos = after_lang.find(' ')?;

        let language = after_lang[..space_pos].trim().to_string();
        if language.is_empty() {
            return None;
        }

        // Extract the quoted pattern
        let rest = after_lang[space_pos..].trim();

        // Pattern must be in quotes
        if !rest.starts_with('"') {
            return None;
        }

        // Find the closing quote (handle escaped quotes)
        let pattern_str = &rest[1..]; // Skip opening quote
        let mut end_pos = None;
        let mut chars = pattern_str.char_indices().peekable();

        while let Some((i, ch)) = chars.next() {
            if ch == '\\' {
                // Skip the next character (escaped)
                chars.next();
                continue;
            }
            if ch == '"' {
                end_pos = Some(i);
                break;
            }
        }

        let end_pos = end_pos?;
        let pattern = pattern_str[..end_pos].replace("\\\"", "\"");

        Some(Self::new(language, pattern, value.to_string()))
    }

    /// Get the language for xiuxian-ast queries.
    ///
    /// Returns `None` if the language string is not a supported AST language.
    #[must_use]
    pub fn ast_language(&self) -> Option<xiuxian_ast::Lang> {
        xiuxian_ast::Lang::try_from(self.language.as_str()).ok()
    }

    /// Validate the pattern using xiuxian-ast.
    ///
    /// # Returns
    ///
    /// `Ok(())` if the pattern is valid, `Err(String)` with error message if invalid.
    pub fn validate_pattern(&self) -> Result<(), String> {
        let lang = self
            .ast_language()
            .ok_or_else(|| format!("Unsupported language: {}", self.language))?;

        xiuxian_ast::pattern(&self.pattern, lang).map_err(|e| format!("Invalid pattern: {e}"))?;

        Ok(())
    }
}

impl fmt::Display for CodeObservation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            ":OBSERVE: lang:{} \"{}\"",
            self.language,
            self.pattern.replace('"', "\\\"")
        )
    }
}

/// Extract all `:OBSERVE:` entries from property drawer attributes.
///
/// Supports multiple observation patterns per section by using:
/// - Single `:OBSERVE:` with the full format
/// - Multiple `:OBSERVE:` entries (numbered or repeated)
///
/// # Example
///
/// ```markdown
/// :OBSERVE: lang:rust "fn $NAME($$$) -> Result<$$$>"
/// ```
#[must_use]
pub fn extract_observations(attributes: &HashMap<String, String>) -> Vec<CodeObservation> {
    let mut observations = Vec::new();

    // Check for single :OBSERVE: entry
    if let Some(value) = attributes.get("OBSERVE")
        && let Some(obs) = CodeObservation::parse(value)
    {
        observations.push(obs);
    }

    // Check for numbered entries: :OBSERVE_1:, :OBSERVE_2:, etc.
    for (key, value) in attributes {
        if key.starts_with("OBSERVE_")
            && let Some(obs) = CodeObservation::parse(value)
        {
            observations.push(obs);
        }
    }

    observations
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_pattern() {
        let input = r#"lang:rust "fn $NAME($$$ARGS) -> Result<$$$RET, $$$ERR>""#;
        let obs = CodeObservation::parse(input);
        assert!(obs.is_some());
        let obs = obs.unwrap();
        assert_eq!(obs.language, "rust");
        assert_eq!(obs.pattern, "fn $NAME($$$ARGS) -> Result<$$$RET, $$$ERR>");
    }

    #[test]
    fn test_parse_python_pattern() {
        let input = r#"lang:python "def $NAME($$$): $$$BODY""#;
        let obs = CodeObservation::parse(input);
        assert!(obs.is_some());
        let obs = obs.unwrap();
        assert_eq!(obs.language, "python");
        assert_eq!(obs.pattern, "def $NAME($$$): $$$BODY");
    }

    #[test]
    fn test_parse_with_escaped_quotes() {
        let input = r#"lang:rust "fn foo() { let s = \"hello\"; }""#;
        let obs = CodeObservation::parse(input);
        assert!(obs.is_some());
        let obs = obs.unwrap();
        assert_eq!(obs.pattern, r#"fn foo() { let s = "hello"; }"#);
    }

    #[test]
    fn test_parse_missing_lang_prefix() {
        let input = r#""fn $NAME()""#;
        assert!(CodeObservation::parse(input).is_none());
    }

    #[test]
    fn test_parse_missing_quotes() {
        let input = r#"lang:rust fn $NAME()"#;
        assert!(CodeObservation::parse(input).is_none());
    }

    #[test]
    fn test_parse_empty_language() {
        let input = r#"lang: "fn $NAME()""#;
        assert!(CodeObservation::parse(input).is_none());
    }

    #[test]
    fn test_ast_language_rust() {
        let obs = CodeObservation::parse(r#"lang:rust "fn main()""#).unwrap();
        assert!(obs.ast_language().is_some());
        assert_eq!(obs.ast_language().unwrap(), xiuxian_ast::Lang::Rust);
    }

    #[test]
    fn test_ast_language_python() {
        let obs = CodeObservation::parse(r#"lang:python "def main():""#).unwrap();
        assert!(obs.ast_language().is_some());
        assert_eq!(obs.ast_language().unwrap(), xiuxian_ast::Lang::Python);
    }

    #[test]
    fn test_ast_language_unsupported() {
        let obs = CodeObservation::parse(r#"lang:brainfuck "+-<>""#).unwrap();
        assert!(obs.ast_language().is_none());
    }

    #[test]
    fn test_validate_pattern_valid() {
        let obs = CodeObservation::parse(r#"lang:rust "fn $NAME()""#).unwrap();
        assert!(obs.validate_pattern().is_ok());
    }

    #[test]
    fn test_validate_pattern_unsupported_lang() {
        let obs = CodeObservation::parse(r#"lang:brainfuck "+-<>""#).unwrap();
        let result = obs.validate_pattern();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Unsupported language"));
    }

    #[test]
    fn test_extract_observations_single() {
        let mut attrs = HashMap::new();
        attrs.insert(
            "OBSERVE".to_string(),
            r#"lang:rust "fn $NAME()""#.to_string(),
        );

        let observations = extract_observations(&attrs);
        assert_eq!(observations.len(), 1);
        assert_eq!(observations[0].language, "rust");
    }

    #[test]
    fn test_extract_observations_multiple() {
        let mut attrs = HashMap::new();
        attrs.insert(
            "OBSERVE_1".to_string(),
            r#"lang:rust "fn $NAME()""#.to_string(),
        );
        attrs.insert(
            "OBSERVE_2".to_string(),
            r#"lang:python "def $NAME():""#.to_string(),
        );

        let observations = extract_observations(&attrs);
        assert_eq!(observations.len(), 2);
    }

    #[test]
    fn test_extract_observations_none() {
        let attrs = HashMap::new();
        let observations = extract_observations(&attrs);
        assert!(observations.is_empty());
    }

    #[test]
    fn test_display() {
        let obs = CodeObservation::parse(r#"lang:rust "fn main()""#).unwrap();
        assert_eq!(obs.to_string(), r#":OBSERVE: lang:rust "fn main()""#);
    }

    #[test]
    fn test_with_line() {
        let obs = CodeObservation::parse(r#"lang:rust "fn main()""#)
            .unwrap()
            .with_line(42);
        assert_eq!(obs.line_number, Some(42));
    }
}
