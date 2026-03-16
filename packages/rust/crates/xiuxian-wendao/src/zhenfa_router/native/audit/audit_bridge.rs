//! Audit Bridge for Batch Fixes (Blueprint v3.0)
//!
//! This module provides the bridge between the semantic auditor and external tools
//! for batch remediation of issues found during auditing.
//!
//! ## Architecture (v3.0 - Surgical Fix Protocol)
//!
//! ```text
//! ┌─────────────┐     ┌──────────────────┐     ┌─────────────────┐
//! │  Auditor    │ --> │   AuditBridge    │ --> │  External Tool  │
//! │ (Rust)      │     │ (BatchFix v3.0)  │     │  (qianji/other) │
//! └─────────────┘     └──────────────────┘     ���─────────────────┘
//!                              │
//!                              ▼
//!                     ┌──────────────────┐
//!                     │  CAS Verification │
//!                     │  (base_hash +    │
//!                     │   byte_range)    │
//!                     └──────────────────┘
//! ```
//!
//! ## Surgical Fix Protocol (v3.0)
//!
//! Unlike v2.9 which searched for content strings, v3.0 uses precise
//! byte-addressable operations with content fingerprint verification:
//!
//! 1. **base_hash**: SHA-256 hash of file content BEFORE fix
//! 2. **byte_range**: Exact (start, end) byte positions to replace
//! 3. **replacement**: New content to insert
//!
//! This enables "robotic surgeon" precision without reading the full file.
//!
//! ## Usage
//!
//! ```ignore
//! use crate::zhenfa_router::native::audit::{BatchFix, FixResult};
//!
//! // Create a surgical fix with byte precision
//! let fix = BatchFix::surgical(
//!     "docs/api.md".to_string(),
//!     42,                                    // line_number (for diagnostics)
//!     (100, 150),                            // byte_range
//!     "a1b2c3d4...",                         // base_hash (SHA-256 of file)
//!     ":OBSERVE: lang:rust \"fn process_data\"".to_string(),
//!     ":OBSERVE: lang:rust \"fn process_records\"".to_string(),
//!     0.85,
//! );
//!
//! // Apply with CAS verification
//! let result = fix.apply_surgical(&file_content);
//! assert!(matches!(result, FixResult::Success));
//! ```

use serde::{Deserialize, Serialize};

use super::super::semantic_check::{FuzzySuggestionData, IssueLocation, SemanticIssue};

/// Byte range for precise content addressing.
///
/// Encapsulates `(start, end)` positions to prevent offset calculation errors
/// and provide convenient methods for range operations.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ByteRange {
    /// Start byte position (inclusive).
    pub start: usize,
    /// End byte position (exclusive).
    pub end: usize,
}

impl ByteRange {
    /// Create a new byte range.
    #[must_use]
    pub const fn new(start: usize, end: usize) -> Self {
        Self { start, end }
    }

    /// Get the length of this range.
    #[must_use]
    pub const fn len(&self) -> usize {
        self.end - self.start
    }

    /// Check if this range is empty.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.start >= self.end
    }

    /// Check if a byte position is within this range.
    #[must_use]
    pub const fn contains(&self, pos: usize) -> bool {
        pos >= self.start && pos < self.end
    }

    /// Check if this range overlaps with another.
    #[must_use]
    pub const fn overlaps(&self, other: &Self) -> bool {
        self.start < other.end && other.start < self.end
    }

    /// Adjust this range by a delta (for offset correction after edits).
    #[must_use]
    pub const fn adjust(&self, delta: isize) -> Self {
        Self {
            start: if delta >= 0 {
                self.start.saturating_add(delta as usize)
            } else {
                self.start.saturating_sub((-delta) as usize)
            },
            end: if delta >= 0 {
                self.end.saturating_add(delta as usize)
            } else {
                self.end.saturating_sub((-delta) as usize)
            },
        }
    }

    /// Extract content from a string at this range.
    ///
    /// Returns `None` if the range is out of bounds or invalid UTF-8 boundary.
    #[must_use]
    pub fn extract<'a>(&self, content: &'a str) -> Option<&'a str> {
        let bytes = content.as_bytes();
        if !self.is_valid_for(bytes.len()) {
            return None;
        }
        // Safe: We verified bounds above, and since we're slicing a valid UTF-8 string
        // at valid byte boundaries, the result is valid UTF-8
        content.get(self.start..self.end)
    }

    /// Check if this range is valid for a given content length.
    #[must_use]
    pub const fn is_valid_for(&self, len: usize) -> bool {
        self.start <= self.end && self.end <= len
    }
}

impl From<(usize, usize)> for ByteRange {
    fn from((start, end): (usize, usize)) -> Self {
        Self::new(start, end)
    }
}

impl From<ByteRange> for (usize, usize) {
    fn from(range: ByteRange) -> Self {
        (range.start, range.end)
    }
}

impl std::fmt::Display for ByteRange {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}:{}", self.start, self.end)
    }
}

/// Result of applying a surgical fix.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FixResult {
    /// Fix was applied successfully.
    Success,
    /// Hash mismatch - file was modified since audit.
    HashMismatch {
        /// Expected hash (from audit time).
        expected: String,
        /// Actual hash (current file state).
        actual: String,
    },
    /// Byte range out of bounds.
    OutOfBounds {
        /// Requested range.
        range: ByteRange,
        /// File size in bytes.
        file_size: usize,
    },
    /// Content at byte range doesn't match expected original.
    ContentMismatch {
        /// Expected original content.
        expected: String,
        /// Actual content at range.
        actual: String,
    },
}

impl std::fmt::Display for FixResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Success => write!(f, "Fix applied successfully"),
            Self::HashMismatch { expected, actual } => {
                write!(
                    f,
                    "Hash mismatch: expected {}..8, got {}..8",
                    &expected[..8.min(expected.len())],
                    &actual[..8.min(actual.len())]
                )
            }
            Self::OutOfBounds { range, file_size } => {
                write!(f, "Byte range {} exceeds file size {}", range, file_size)
            }
            Self::ContentMismatch { expected, actual } => {
                write!(
                    f,
                    "Content mismatch at byte range: expected {:?}, got {:?}",
                    expected.chars().take(50).collect::<String>(),
                    actual.chars().take(50).collect::<String>()
                )
            }
        }
    }
}

/// Represents a single fix operation to be applied to a document.
///
/// ## Version History
///
/// - **v2.9**: String-based search and replace
/// - **v3.0**: Byte-addressable with CAS verification (surgical precision)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchFix {
    /// Type of issue being fixed.
    pub issue_type: String,
    /// Document path where the fix should be applied.
    pub doc_path: String,
    /// Line number where the fix should be applied (for diagnostics).
    pub line_number: usize,
    /// Original content to be replaced (for verification and fallback).
    pub original_content: String,
    /// Replacement content (e.g., updated property drawer).
    pub replacement: String,
    /// Confidence score for this fix (0.0 - 1.0).
    pub confidence: f32,
    /// Source location where similar code was found (if applicable).
    pub source_location: Option<String>,

    // === v3.0 Surgical Fix Fields ===
    /// SHA-256 hash of file content at audit time.
    /// Used for optimistic concurrency control (CAS verification).
    pub base_hash: Option<String>,
    /// Precise byte range (start, end) for the content to replace.
    /// Enables O(1) positioning without string search.
    pub byte_range: Option<ByteRange>,
}

impl BatchFix {
    /// Create a new batch fix (v2.9 compatible).
    #[must_use]
    pub fn new(
        issue_type: String,
        doc_path: String,
        line_number: usize,
        original_content: String,
        replacement: String,
        confidence: f32,
    ) -> Self {
        Self {
            issue_type,
            doc_path,
            line_number,
            original_content,
            replacement,
            confidence,
            source_location: None,
            base_hash: None,
            byte_range: None,
        }
    }

    /// Create a surgical fix with byte precision (v3.0).
    #[must_use]
    pub fn surgical(
        doc_path: String,
        line_number: usize,
        byte_range: ByteRange,
        base_hash: String,
        original_content: String,
        replacement: String,
        confidence: f32,
    ) -> Self {
        Self {
            issue_type: "invalid_observation_pattern".to_string(),
            doc_path,
            line_number,
            original_content,
            replacement,
            confidence,
            source_location: None,
            base_hash: Some(base_hash),
            byte_range: Some(byte_range),
        }
    }

    /// Create a batch fix with source location.
    #[must_use]
    pub fn with_source(mut self, source_location: String) -> Self {
        self.source_location = Some(source_location);
        self
    }

    /// Create a batch fix with surgical precision data.
    #[must_use]
    pub fn with_surgical(mut self, byte_range: ByteRange, base_hash: String) -> Self {
        self.byte_range = Some(byte_range);
        self.base_hash = Some(base_hash);
        self
    }

    /// Generate a batch fix from a fuzzy suggestion.
    #[must_use]
    pub fn from_fuzzy_suggestion(
        doc_path: String,
        line_number: usize,
        original_content: String,
        suggestion: &FuzzySuggestionData,
    ) -> Self {
        Self {
            issue_type: "invalid_observation_pattern".to_string(),
            doc_path,
            line_number,
            original_content,
            replacement: suggestion.replacement_drawer.clone(),
            confidence: suggestion.confidence,
            source_location: suggestion.source_location.clone(),
            base_hash: None,
            byte_range: None,
        }
    }

    /// Generate a surgical batch fix from a fuzzy suggestion with location data.
    #[must_use]
    pub fn from_fuzzy_suggestion_surgical(
        doc_path: String,
        location: &IssueLocation,
        _original_content: String,
        file_content: &str,
        suggestion: &FuzzySuggestionData,
    ) -> Self {
        let base_hash = compute_hash(file_content);

        // v3.3 Immutable Cursor Algorithm
        // Find the exact byte offsets of the target line by iterating through line boundaries.
        let mut target_range: Option<ByteRange> = None;
        let mut actual_raw_line = String::new();

        let mut current_pos = 0;
        for (i, line_content) in file_content.split_inclusive('\n').enumerate() {
            let line_num = i + 1;
            let line_len = line_content.len();

            if line_num == location.line {
                // Check if this line contains our observation markers
                if line_content.contains(":OBSERVE:")
                    || line_content.contains(&suggestion.original_pattern)
                {
                    target_range = Some(ByteRange::new(current_pos, current_pos + line_len));
                    actual_raw_line = line_content.to_string();
                    break;
                }
            }
            current_pos += line_len;
        }

        // Handle case where line is exactly at EOF without trailing newline
        if target_range.is_none() && location.line > 0 {
            let mut current_pos = 0;
            for (i, line_content) in file_content.lines().enumerate() {
                let line_num = i + 1;
                let line_len = line_content.len();
                if line_num == location.line {
                    // Check if it's the last line
                    if current_pos + line_len == file_content.len() {
                        target_range = Some(ByteRange::new(current_pos, file_content.len()));
                        actual_raw_line = line_content.to_string();
                        break;
                    }
                }
                current_pos += line_len + 1; // Brute-force assumption for fallback
            }
        }

        if let Some(range) = target_range {
            // Ensure the replacement also maintains the exact line-ending structure of the source
            let mut replacement = suggestion.replacement_drawer.clone();
            if actual_raw_line.ends_with('\n') && !replacement.ends_with('\n') {
                replacement.push('\n');
            } else if actual_raw_line.ends_with("\r\n") && !replacement.ends_with("\r\n") {
                replacement.push_str("\r\n");
            }

            return Self::surgical(
                doc_path,
                location.line,
                range,
                base_hash,
                actual_raw_line,
                replacement,
                suggestion.confidence,
            )
            .with_source(suggestion.source_location.clone().unwrap_or_default());
        }

        // Ultimate Fallback: use provided metadata
        let fallback_range = location
            .byte_range
            .map(|(s, e)| ByteRange::new(s, e))
            .unwrap_or_else(|| ByteRange::new(0, 0));

        Self::surgical(
            doc_path,
            location.line,
            fallback_range,
            base_hash,
            _original_content,
            suggestion.replacement_drawer.clone(),
            suggestion.confidence,
        )
        .with_source(suggestion.source_location.clone().unwrap_or_default())
    }

    /// Check if this fix has surgical precision data.
    #[must_use]
    pub fn is_surgical(&self) -> bool {
        self.base_hash.is_some() && self.byte_range.is_some()
    }

    /// Apply the fix using surgical precision (v3.1).
    ///
    /// This method uses byte-range addressing for precise, safe content modification.
    /// Hash verification is NOT performed internally - use `AtomicFixBatch::apply_all`
    /// for one-time hash verification before applying multiple fixes.
    ///
    /// # Arguments
    ///
    /// * `content` - The file content to modify
    ///
    /// # Returns
    ///
    /// - `FixResult::Success` if the fix was applied
    /// - `FixResult::OutOfBounds` if byte range exceeds file size
    /// - `FixResult::ContentMismatch` if content at range doesn't match expected
    pub fn apply_surgical(&self, content: &mut String) -> FixResult {
        // Get byte range (fallback to string search if not available)
        let range = match self.byte_range {
            Some(range) => range,
            None => {
                // Fallback to v2.9 string search
                return self.apply_legacy(content);
            }
        };

        // Verify byte range is valid
        let content_bytes = content.as_bytes();
        if !range.is_valid_for(content_bytes.len()) {
            return FixResult::OutOfBounds {
                range,
                file_size: content_bytes.len(),
            };
        }

        // Verify content at range matches expected
        let actual_at_range = range.extract(content).unwrap_or("");
        if actual_at_range != self.original_content {
            return FixResult::ContentMismatch {
                expected: self.original_content.clone(),
                actual: actual_at_range.to_string(),
            };
        }

        // Perform the replacement
        content.replace_range(range.start..range.end, &self.replacement);

        FixResult::Success
    }

    /// Apply the fix using legacy string search (v2.9).
    ///
    /// Fallback method when surgical data is not available.
    fn apply_legacy(&self, content: &mut String) -> FixResult {
        // Find the original content in the document
        let start_pos = match content.find(&self.original_content) {
            Some(pos) => pos,
            None => {
                return FixResult::ContentMismatch {
                    expected: self.original_content.clone(),
                    actual: "(not found in document)".to_string(),
                };
            }
        };

        // Replace the content
        let end_pos = start_pos + self.original_content.len();
        content.replace_range(start_pos..end_pos, &self.replacement);

        FixResult::Success
    }

    /// Apply the fix to a document content (v2.9 compatible).
    ///
    /// # Returns
    ///
    /// `Ok(())` if the fix was applied successfully.
    /// `Err(String)` if the fix could not be applied.
    pub fn apply(&self, content: &mut String) -> Result<(), String> {
        match self.apply_surgical(content) {
            FixResult::Success => Ok(()),
            other => Err(other.to_string()),
        }
    }

    /// Preview the fix without applying it.
    ///
    /// Returns the content that would result from applying this fix,
    /// or an error if the fix cannot be applied.
    #[must_use]
    pub fn preview(&self, content: &str) -> Result<String, FixResult> {
        let mut content = content.to_string();
        match self.apply_surgical(&mut content) {
            FixResult::Success => Ok(content),
            other => Err(other),
        }
    }
}

/// Compute Blake3 hash of content (3-5x faster than SHA-256).
fn compute_hash(content: &str) -> String {
    let hash = blake3::hash(content.as_bytes());
    hash.to_hex().to_string()
}

/// Trait for bridging audit results to external tools.
pub trait AuditBridge: Send + std::fmt::Debug {
    /// Process audit issues and generate batch fixes.
    fn generate_fixes(&self, issues: &[SemanticIssue]) -> Vec<BatchFix>;

    /// Apply batch fixes using an external tool.
    fn apply_fixes(&self, fixes: &[BatchFix]) -> Result<Vec<String>, Vec<String>>;

    /// Get the name of the bridge (for logging).
    fn name(&self) -> &str;
}

/// Default implementation of AuditBridge that generates fixes but doesn't apply them.
#[derive(Debug, Default)]
pub struct DefaultAuditBridge;

impl AuditBridge for DefaultAuditBridge {
    fn generate_fixes(&self, issues: &[SemanticIssue]) -> Vec<BatchFix> {
        issues
            .iter()
            .filter_map(|issue| {
                // Only process issues with fuzzy suggestions
                issue.fuzzy_suggestion.as_ref().map(|suggestion| {
                    BatchFix::from_fuzzy_suggestion(
                        issue.doc.clone(),
                        issue.location.as_ref().map(|loc| loc.line).unwrap_or(0),
                        issue.suggestion.clone().unwrap_or_default(),
                        suggestion,
                    )
                })
            })
            .collect()
    }

    fn apply_fixes(&self, _fixes: &[BatchFix]) -> Result<Vec<String>, Vec<String>> {
        // Default implementation doesn't apply fixes
        // Returns empty success list
        Ok(Vec::new())
    }

    fn name(&self) -> &str {
        "default"
    }
}

/// Generate batch fixes from semantic check issues.
///
/// # Arguments
///
/// * `issues` - List of semantic issues from the auditor
///
/// # Returns
///
/// A list of `BatchFix` operations that can be applied
#[must_use]
pub fn generate_batch_fixes(issues: &[SemanticIssue]) -> Vec<BatchFix> {
    let bridge = DefaultAuditBridge;
    bridge.generate_fixes(issues)
}

/// Generate surgical batch fixes with byte precision.
///
/// This function requires file content to compute base_hash and byte ranges.
///
/// # Arguments
///
/// * `issues` - List of semantic issues from the auditor
/// * `file_contents` - Map of doc_path -> file content for computing hashes
///
/// # Returns
///
/// A list of surgical `BatchFix` operations with byte precision
#[must_use]
pub fn generate_surgical_fixes(
    issues: &[SemanticIssue],
    file_contents: &std::collections::HashMap<String, String>,
) -> Vec<BatchFix> {
    issues
        .iter()
        .filter_map(|issue| {
            // Only process issues with fuzzy suggestions and location
            let suggestion = issue.fuzzy_suggestion.as_ref()?;
            let location = issue.location.as_ref()?;

            // Get file content for hash computation
            let file_content = file_contents.get(&issue.doc)?;

            Some(BatchFix::from_fuzzy_suggestion_surgical(
                issue.doc.clone(),
                location,
                issue.suggestion.clone().unwrap_or_default(),
                file_content,
                suggestion,
            ))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::zhenfa_router::native::semantic_check::{
        FuzzySuggestionData, IssueLocation, SemanticIssue,
    };

    fn test_file_content() -> String {
        "line 1\n:OBSERVE: lang:rust \"fn process_data\"\nline 3".to_string()
    }

    /// Get the correct byte range for the OBSERVE line in test_file_content.
    fn observe_line_range() -> ByteRange {
        // Test content: "line 1\n:OBSERVE: lang:rust \"fn process_data\"\nline 3"
        // Let's calculate byte positions:
        // Position 0-6: "line 1\n" (7 bytes)
        // Position 7-44: ":OBSERVE: lang:rust \"fn process_data\"" (38 bytes)
        // Position 45: '\n'
        // So to get just the OBSERVE without newline: (7, 45)
        let content = test_file_content();
        let observe_content = r#":OBSERVE: lang:rust "fn process_data""#;
        let start = content
            .find(observe_content)
            .expect("OBSERVE content should exist");
        let end = start + observe_content.len();
        ByteRange::new(start, end)
    }

    #[test]
    fn test_batch_fix_from_fuzzy_suggestion() {
        let suggestion = FuzzySuggestionData {
            original_pattern: "fn process_data($$$)".to_string(),
            suggested_pattern: "fn process_records($$$)".to_string(),
            confidence: 0.85,
            source_location: Some("src/lib.rs:42".to_string()),
            replacement_drawer: r#":OBSERVE: lang:rust "fn process_records($$$)""#.to_string(),
        };

        let fix = BatchFix::from_fuzzy_suggestion(
            "docs/api.md".to_string(),
            42,
            r#":OBSERVE: lang:rust "fn process_data($$$)""#.to_string(),
            &suggestion,
        );

        assert_eq!(fix.issue_type, "invalid_observation_pattern");
        assert_eq!(fix.doc_path, "docs/api.md");
        assert_eq!(fix.line_number, 42);
        assert!(!fix.is_surgical());
    }

    #[test]
    fn test_batch_fix_surgical() {
        let content = test_file_content();
        let base_hash = compute_hash(&content);
        let byte_range = observe_line_range();

        let fix = BatchFix::surgical(
            "test.md".to_string(),
            2,
            byte_range,
            base_hash,
            r#":OBSERVE: lang:rust "fn process_data""#.to_string(),
            r#":OBSERVE: lang:rust "fn process_records""#.to_string(),
            0.9,
        );

        assert!(fix.is_surgical());
        assert!(fix.byte_range.is_some());
        assert!(fix.base_hash.is_some());
    }

    #[test]
    fn test_surgical_fix_apply_success() {
        let content = test_file_content();
        let base_hash = compute_hash(&content);
        let byte_range = observe_line_range();

        let fix = BatchFix::surgical(
            "test.md".to_string(),
            2,
            byte_range,
            base_hash,
            r#":OBSERVE: lang:rust "fn process_data""#.to_string(),
            r#":OBSERVE: lang:rust "fn process_records""#.to_string(),
            0.9,
        );

        let mut content = test_file_content();
        let result = fix.apply_surgical(&mut content);

        assert_eq!(result, FixResult::Success);
        assert!(content.contains("process_records"));
    }

    #[test]
    fn test_surgical_fix_content_at_range_mismatch() {
        // v3.1: Hash verification is now external (one-time in AtomicFixBatch)
        // This test verifies that content mismatch at byte range is detected
        let content = test_file_content();
        let base_hash = compute_hash(&content);
        let byte_range = ByteRange::new(0, 3);

        let fix = BatchFix::surgical(
            "test.md".to_string(),
            1,
            byte_range,
            base_hash,
            "old".to_string(), // Wrong - actual content starts with "lin"
            "new".to_string(),
            0.9,
        );

        let mut content = test_file_content();
        let result = fix.apply_surgical(&mut content);

        // Content at range (0, 3) is "lin", not "old" -> ContentMismatch
        assert!(matches!(result, FixResult::ContentMismatch { .. }));
    }

    #[test]
    fn test_surgical_fix_out_of_bounds() {
        let content = test_file_content();
        let base_hash = compute_hash(&content);
        let byte_range = ByteRange::new(100, 200); // Out of bounds

        let fix = BatchFix::surgical(
            "test.md".to_string(),
            2,
            byte_range,
            base_hash,
            "something".to_string(),
            "replacement".to_string(),
            0.9,
        );

        let mut content = test_file_content();
        let result = fix.apply_surgical(&mut content);

        assert!(matches!(result, FixResult::OutOfBounds { .. }));
    }

    #[test]
    fn test_surgical_fix_content_mismatch() {
        let content = test_file_content();
        let base_hash = compute_hash(&content);
        let byte_range = observe_line_range();

        let fix = BatchFix::surgical(
            "test.md".to_string(),
            2,
            byte_range,
            base_hash,
            "wrong original content".to_string(), // Doesn't match actual content at range
            "replacement".to_string(),
            0.9,
        );

        let mut content = test_file_content();
        let result = fix.apply_surgical(&mut content);

        assert!(matches!(result, FixResult::ContentMismatch { .. }));
    }

    #[test]
    fn test_legacy_fallback() {
        let fix = BatchFix::new(
            "test".to_string(),
            "test.md".to_string(),
            1,
            "old content".to_string(),
            "new content".to_string(),
            0.9,
        );

        let mut content = "line 1\nold content\nline 3".to_string();
        let result = fix.apply_surgical(&mut content);

        assert_eq!(result, FixResult::Success);
        assert_eq!(content, "line 1\nnew content\nline 3");
    }

    #[test]
    fn test_preview() {
        let content = test_file_content();
        let base_hash = compute_hash(&content);
        let byte_range = observe_line_range();

        let fix = BatchFix::surgical(
            "test.md".to_string(),
            2,
            byte_range,
            base_hash,
            r#":OBSERVE: lang:rust "fn process_data""#.to_string(),
            r#":OBSERVE: lang:rust "fn process_records""#.to_string(),
            0.9,
        );

        let original = test_file_content();
        let preview = fix.preview(&original).unwrap();

        assert!(preview.contains("process_records"));
        assert!(!original.contains("process_records")); // Original unchanged
    }

    #[test]
    fn test_compute_hash_deterministic() {
        let content = "test content";
        let hash1 = compute_hash(content);
        let hash2 = compute_hash(content);

        assert_eq!(hash1, hash2);
        assert_eq!(hash1.len(), 64); // SHA-256 hex length
    }

    #[test]
    fn test_with_surgical() {
        let content = test_file_content();
        let base_hash = compute_hash(&content);

        let fix = BatchFix::new(
            "test".to_string(),
            "test.md".to_string(),
            2,
            "old".to_string(),
            "new".to_string(),
            0.9,
        )
        .with_surgical(ByteRange::new(7, 43), base_hash);

        assert!(fix.is_surgical());
    }

    #[test]
    fn test_generate_surgical_fixes() {
        let doc_path = "docs/api.md".to_string();
        let file_content = "line 1\n:OBSERVE: lang:rust \"fn process_data\"\nline 3".to_string();

        let mut file_contents = std::collections::HashMap::new();
        file_contents.insert(doc_path.clone(), file_content.clone());

        let issues = vec![SemanticIssue {
            severity: "error".to_string(),
            issue_type: "invalid_observation_pattern".to_string(),
            doc: doc_path.clone(),
            node_id: "node1".to_string(),
            message: "Invalid pattern".to_string(),
            location: Some(IssueLocation {
                line: 2,
                heading_path: "API".to_string(),
                byte_range: Some((7, 43)),
            }),
            suggestion: Some(":OBSERVE: lang:rust \"fn process_data\"".to_string()),
            fuzzy_suggestion: Some(FuzzySuggestionData {
                original_pattern: "fn process_data".to_string(),
                suggested_pattern: "fn process_records($$$)".to_string(),
                confidence: 0.85,
                source_location: Some("src/lib.rs:42".to_string()),
                replacement_drawer: r#":OBSERVE: lang:rust "fn process_records($$$)""#.to_string(),
            }),
        }];

        let fixes = generate_surgical_fixes(&issues, &file_contents);

        assert_eq!(fixes.len(), 1);
        assert!(fixes[0].is_surgical());
        assert!(fixes[0].base_hash.is_some());
        assert_eq!(fixes[0].byte_range, Some(ByteRange::new(7, 45))); // Full line including newline
    }

    // =============================================================================
    // FixResult Display Tests
    // =============================================================================

    #[test]
    fn test_fix_result_display_success() {
        let result = FixResult::Success;
        assert_eq!(format!("{}", result), "Fix applied successfully");
    }

    #[test]
    fn test_fix_result_display_hash_mismatch() {
        let result = FixResult::HashMismatch {
            expected: "a1b2c3d4e5f6".to_string(),
            actual: "x1y2z3a4b5c6".to_string(),
        };
        let display = format!("{}", result);
        assert!(display.contains("Hash mismatch"));
        assert!(display.contains("a1b2c3d4"));
        assert!(display.contains("x1y2z3a4"));
    }

    #[test]
    fn test_fix_result_display_out_of_bounds() {
        let result = FixResult::OutOfBounds {
            range: ByteRange::new(100, 200),
            file_size: 50,
        };
        let display = format!("{}", result);
        assert!(display.contains("Byte range"));
        assert!(display.contains("exceeds file size"));
        assert!(display.contains("50"));
    }

    #[test]
    fn test_fix_result_display_content_mismatch() {
        let result = FixResult::ContentMismatch {
            expected: "expected content".to_string(),
            actual: "actual content".to_string(),
        };
        let display = format!("{}", result);
        assert!(display.contains("Content mismatch"));
        assert!(display.contains("expected"));
        assert!(display.contains("actual"));
    }

    // =============================================================================
    // AuditBridge Trait Tests
    // =============================================================================

    #[test]
    fn test_default_audit_bridge_generate_fixes() {
        let bridge = DefaultAuditBridge;

        let issues = vec![
            SemanticIssue {
                severity: "error".to_string(),
                issue_type: "invalid_observation_pattern".to_string(),
                doc: "docs/api.md".to_string(),
                node_id: "node1".to_string(),
                message: "Invalid pattern".to_string(),
                location: Some(IssueLocation {
                    line: 42,
                    heading_path: "API".to_string(),
                    byte_range: None,
                }),
                suggestion: Some("Fix it".to_string()),
                fuzzy_suggestion: Some(FuzzySuggestionData {
                    original_pattern: "fn process_data".to_string(),
                    suggested_pattern: "fn process_records($$$)".to_string(),
                    confidence: 0.85,
                    source_location: Some("src/lib.rs:42".to_string()),
                    replacement_drawer: r#":OBSERVE: lang:rust "fn process_records($$$)""#
                        .to_string(),
                }),
            },
            // Issue without fuzzy_suggestion - should be filtered out
            SemanticIssue {
                severity: "error".to_string(),
                issue_type: "dead_link".to_string(),
                doc: "docs/other.md".to_string(),
                node_id: "node2".to_string(),
                message: "Dead link".to_string(),
                location: None,
                suggestion: None,
                fuzzy_suggestion: None,
            },
        ];

        let fixes = bridge.generate_fixes(&issues);

        // Only one fix should be generated (for the issue with fuzzy_suggestion)
        assert_eq!(fixes.len(), 1);
        assert_eq!(fixes[0].doc_path, "docs/api.md");
        assert_eq!(bridge.name(), "default");
    }

    #[test]
    fn test_default_audit_bridge_apply_fixes() {
        let bridge = DefaultAuditBridge;

        let fixes = vec![BatchFix::new(
            "test".to_string(),
            "test.md".to_string(),
            1,
            "old".to_string(),
            "new".to_string(),
            0.9,
        )];

        // Default implementation doesn't apply fixes, returns empty success list
        let result = bridge.apply_fixes(&fixes);
        assert!(result.is_ok());
        assert!(result.unwrap().is_empty());
    }

    #[test]
    fn test_generate_batch_fixes_function() {
        let issues = vec![SemanticIssue {
            severity: "error".to_string(),
            issue_type: "invalid_observation_pattern".to_string(),
            doc: "docs/api.md".to_string(),
            node_id: "node1".to_string(),
            message: "Invalid pattern".to_string(),
            location: None,
            suggestion: Some("Fix it".to_string()),
            fuzzy_suggestion: Some(FuzzySuggestionData {
                original_pattern: "fn process_data".to_string(),
                suggested_pattern: "fn process_records($$$)".to_string(),
                confidence: 0.85,
                source_location: Some("src/lib.rs:42".to_string()),
                replacement_drawer: r#":OBSERVE: lang:rust "fn process_records($$$)""#.to_string(),
            }),
        }];

        let fixes = generate_batch_fixes(&issues);
        assert_eq!(fixes.len(), 1);
    }

    // =============================================================================
    // Edge Cases and Boundary Tests
    // =============================================================================

    #[test]
    fn test_surgical_fix_empty_content() {
        let content = "";
        let base_hash = compute_hash(content);
        let byte_range = ByteRange::new(0, 0);

        let fix = BatchFix::surgical(
            "test.md".to_string(),
            1,
            byte_range,
            base_hash,
            "".to_string(),
            "new content".to_string(),
            0.9,
        );

        let mut content = "".to_string();
        let result = fix.apply_surgical(&mut content);

        assert_eq!(result, FixResult::Success);
        assert_eq!(content, "new content");
    }

    #[test]
    fn test_surgical_fix_same_content_replacement() {
        let content = test_file_content();
        let base_hash = compute_hash(&content);
        let byte_range = observe_line_range();
        let observe_content = r#":OBSERVE: lang:rust "fn process_data""#;

        // Replace with same content
        let fix = BatchFix::surgical(
            "test.md".to_string(),
            2,
            byte_range,
            base_hash,
            observe_content.to_string(),
            observe_content.to_string(), // Same as original
            0.9,
        );

        let mut content = test_file_content();
        let result = fix.apply_surgical(&mut content);

        assert_eq!(result, FixResult::Success);
        // Content should be unchanged
        assert_eq!(content, test_file_content());
    }

    #[test]
    fn test_surgical_fix_byte_range_at_file_boundary() {
        let content = "test content";
        let base_hash = compute_hash(content);
        let byte_range = ByteRange::new(0, 12); // Exact file length

        let fix = BatchFix::surgical(
            "test.md".to_string(),
            1,
            byte_range,
            base_hash,
            "test content".to_string(),
            "replaced all".to_string(),
            0.9,
        );

        let mut content = "test content".to_string();
        let result = fix.apply_surgical(&mut content);

        assert_eq!(result, FixResult::Success);
        assert_eq!(content, "replaced all");
    }

    #[test]
    fn test_surgical_fix_start_equals_end() {
        let content = test_file_content();
        let base_hash = compute_hash(&content);
        let byte_range = ByteRange::new(7, 7); // Zero-width range

        let fix = BatchFix::surgical(
            "test.md".to_string(),
            2,
            byte_range,
            base_hash,
            "".to_string(), // Empty original
            "inserted".to_string(),
            0.9,
        );

        let mut content = test_file_content();
        let result = fix.apply_surgical(&mut content);

        // Should succeed - inserting at position 7
        assert_eq!(result, FixResult::Success);
        assert!(content.contains("inserted"));
    }

    #[test]
    fn test_legacy_fallback_not_found() {
        let fix = BatchFix::new(
            "test".to_string(),
            "test.md".to_string(),
            1,
            "nonexistent".to_string(),
            "new".to_string(),
            0.9,
        );

        let mut content = "some other content".to_string();
        let result = fix.apply_surgical(&mut content);

        assert!(matches!(result, FixResult::ContentMismatch { .. }));
    }

    #[test]
    fn test_preview_error() {
        // v3.1: Hash verification is now external (one-time in AtomicFixBatch)
        // This test verifies that content mismatch is detected during preview
        let content = "different content";
        let base_hash = compute_hash(content);

        let fix = BatchFix::surgical(
            "test.md".to_string(),
            1,
            ByteRange::new(0, 3),
            base_hash,
            "old".to_string(), // Wrong - actual content at (0, 3) is "dif"
            "new".to_string(),
            0.9,
        );

        let result = fix.preview(content);

        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            FixResult::ContentMismatch { .. }
        ));
    }

    #[test]
    fn test_surgical_fix_multibyte_utf8() {
        // Test with UTF-8 content containing multibyte characters
        let content = "line 1\n:OBSERVE: lang:rust \"fn 处理数据\"\nline 3";
        let base_hash = compute_hash(content);

        // Find the byte position of the OBSERVE line
        let observe = r#":OBSERVE: lang:rust "fn 处理数据""#;
        let start = content.find(observe).unwrap();
        let end = start + observe.len();

        let fix = BatchFix::surgical(
            "test.md".to_string(),
            2,
            ByteRange::new(start, end),
            base_hash,
            observe.to_string(),
            r#":OBSERVE: lang:rust "fn process_data""#.to_string(),
            0.9,
        );

        let mut content = content.to_string();
        let result = fix.apply_surgical(&mut content);

        assert_eq!(result, FixResult::Success);
        assert!(content.contains("process_data"));
    }

    #[test]
    fn test_is_surgical_method() {
        // Non-surgical fix
        let non_surgical = BatchFix::new(
            "test".to_string(),
            "test.md".to_string(),
            1,
            "old".to_string(),
            "new".to_string(),
            0.9,
        );
        assert!(!non_surgical.is_surgical());

        // Surgical fix
        let surgical = BatchFix::surgical(
            "test.md".to_string(),
            1,
            ByteRange::new(0, 3),
            "hash".to_string(),
            "old".to_string(),
            "new".to_string(),
            0.9,
        );
        assert!(surgical.is_surgical());

        // Partial surgical (only byte_range, no base_hash)
        let partial = BatchFix {
            issue_type: "test".to_string(),
            doc_path: "test.md".to_string(),
            line_number: 1,
            original_content: "old".to_string(),
            replacement: "new".to_string(),
            confidence: 0.9,
            source_location: None,
            base_hash: None,
            byte_range: Some(ByteRange::new(0, 3)),
        };
        assert!(!partial.is_surgical()); // Both fields required
    }
}
