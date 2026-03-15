use std::collections::HashMap;

/// Parsed section row for section-aware retrieval and `HippoRAG 2` `Passage Nodes`.
#[derive(Debug, Clone)]
pub struct ParsedSection {
    /// Leaf heading title for this section.
    pub heading_title: String,
    /// Slash-delimited heading ancestry for this section.
    pub heading_path: String,
    /// Lower-cased `heading_path` for case-insensitive matching.
    pub heading_path_lower: String,
    /// Markdown heading depth for this section.
    pub heading_level: usize,
    /// Inclusive 1-based start line within the markdown body.
    pub line_start: usize,
    /// Inclusive 1-based end line within the markdown body.
    pub line_end: usize,
    /// Byte offset from start of document where this section begins.
    pub byte_start: usize,
    /// Byte offset (exclusive) where this section ends.
    pub byte_end: usize,
    /// Content contained by this section.
    pub section_text: String,
    /// Lower-cased section text for case-insensitive matching.
    pub section_text_lower: String,
    /// List of entity IDs mentioned in this specific section.
    pub entities: Vec<String>,
    /// Property drawer attributes extracted from heading (e.g., :ID: arch-v1).
    pub attributes: std::collections::HashMap<String, String>,
}

#[derive(Clone, Copy)]
struct SectionCursor<'a> {
    heading_title: &'a str,
    heading_path: &'a str,
    heading_level: usize,
    line_range: (usize, usize),
    byte_range: (usize, usize),
}

fn normalize_whitespace(raw: &str) -> String {
    raw.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Parse a property drawer line in the format `:KEY: VALUE`.
///
/// Property drawers must appear immediately after a heading and use the syntax:
/// ```markdown
/// ## Heading
/// :ID: arch-v1
/// :TAGS: core, design
/// ```
fn parse_property_drawer(line: &str) -> Option<(String, String)> {
    let trimmed = line.trim();
    if !trimmed.starts_with(':') {
        return None;
    }

    // Find the closing colon after the key
    let rest = &trimmed[1..]; // Skip the leading ':'
    let colon_pos = rest.find(':')?;

    let key = rest[..colon_pos].trim().to_uppercase();
    if key.is_empty() {
        return None;
    }

    let value = rest[colon_pos + 1..].trim().to_string();
    if value.is_empty() {
        return None;
    }

    Some((key, value))
}

/// Extract property drawer attributes from lines following a heading.
///
/// Returns (attributes, consumed_line_count) where consumed_line_count is the
/// number of lines that were property drawer entries.
fn extract_property_drawers(lines: &[String]) -> HashMap<String, String> {
    let mut attributes = HashMap::new();

    for line in lines {
        if let Some((key, value)) = parse_property_drawer(line) {
            attributes.insert(key, value);
        } else if line.trim().is_empty() {
            // Skip empty lines at the start of the section
            continue;
        } else {
            // Stop at first non-property line
            break;
        }
    }

    attributes
}

fn parse_markdown_heading(line: &str) -> Option<(usize, String)> {
    let trimmed = line.trim_start();
    if !trimmed.starts_with('#') {
        return None;
    }
    let mut level = 0usize;
    for ch in trimmed.chars() {
        if ch == '#' {
            level += 1;
        } else {
            break;
        }
    }
    if level == 0 || level > 6 {
        return None;
    }
    let rest = trimmed[level..].trim_start();
    if rest.is_empty() {
        return None;
    }
    Some((level, normalize_whitespace(rest)))
}

fn push_section(
    out: &mut Vec<ParsedSection>,
    cursor: SectionCursor<'_>,
    lines: &[String],
    source_path: &std::path::Path,
    root: &std::path::Path,
) {
    let section_text = lines.join("\n").trim().to_string();
    if section_text.is_empty() && cursor.heading_path.trim().is_empty() {
        return;
    }

    // Extract property drawer attributes from lines following a heading
    let attributes = if cursor.heading_level > 0 {
        extract_property_drawers(lines)
    } else {
        HashMap::new()
    };

    let extracted = super::links::extract_link_targets(&section_text, source_path, root);
    let line_start = cursor.line_range.0.max(1);
    let line_end = cursor.line_range.1.max(line_start);

    out.push(ParsedSection {
        heading_title: cursor.heading_title.to_string(),
        heading_path: cursor.heading_path.to_string(),
        heading_path_lower: cursor.heading_path.to_lowercase(),
        heading_level: cursor.heading_level,
        line_start,
        line_end,
        byte_start: cursor.byte_range.0,
        byte_end: cursor.byte_range.1,
        section_text_lower: section_text.to_lowercase(),
        section_text,
        entities: extracted.note_links,
        attributes,
    });
}

pub(super) fn extract_sections(
    body: &str,
    source_path: &std::path::Path,
    root: &std::path::Path,
) -> Vec<ParsedSection> {
    let mut sections = Vec::new();
    let mut heading_stack: Vec<String> = Vec::new();
    let mut current_heading_title = String::new();
    let mut current_heading_path = String::new();
    let mut current_heading_level = 0usize;
    let mut current_start_line = 1usize;
    let mut current_start_byte = 0usize;
    let mut current_lines = Vec::new();
    let mut in_code_fence = false;
    let mut last_seen_line = 0usize;
    let mut last_seen_byte = 0usize;

    // Track byte positions while iterating
    let mut byte_offset = 0usize;

    for (line_idx, line) in body.lines().enumerate() {
        let line_no = line_idx + 1;
        let line_bytes = line.len();
        last_seen_line = line_no;
        last_seen_byte = byte_offset + line_bytes;

        let trimmed = line.trim_start();
        if trimmed.starts_with("```") || trimmed.starts_with("~~~") {
            in_code_fence = !in_code_fence;
            current_lines.push(line.to_string());
            byte_offset += line_bytes + 1; // +1 for newline
            continue;
        }
        if !in_code_fence && let Some((level, heading)) = parse_markdown_heading(trimmed) {
            push_section(
                &mut sections,
                SectionCursor {
                    heading_title: &current_heading_title,
                    heading_path: &current_heading_path,
                    heading_level: current_heading_level,
                    line_range: (
                        current_start_line,
                        line_no.saturating_sub(1).max(current_start_line),
                    ),
                    byte_range: (
                        current_start_byte,
                        byte_offset.saturating_sub(1).max(current_start_byte),
                    ),
                },
                &current_lines,
                source_path,
                root,
            );
            current_lines.clear();
            if heading_stack.len() >= level {
                heading_stack.truncate(level.saturating_sub(1));
            }
            heading_stack.push(heading.clone());
            current_heading_title = heading;
            current_heading_path = heading_stack.join(" / ");
            current_heading_level = level;
            current_start_line = line_no;
            current_start_byte = byte_offset;
            byte_offset += line_bytes + 1;
            continue;
        }
        current_lines.push(line.to_string());
        byte_offset += line_bytes + 1;
    }

    push_section(
        &mut sections,
        SectionCursor {
            heading_title: &current_heading_title,
            heading_path: &current_heading_path,
            heading_level: current_heading_level,
            line_range: (current_start_line, last_seen_line.max(current_start_line)),
            byte_range: (current_start_byte, last_seen_byte.max(current_start_byte)),
        },
        &current_lines,
        source_path,
        root,
    );
    if sections.is_empty() {
        let section_text = body.trim().to_string();
        let extracted = super::links::extract_link_targets(&section_text, source_path, root);
        sections.push(ParsedSection {
            heading_title: String::new(),
            heading_path: String::new(),
            heading_path_lower: String::new(),
            heading_level: 0,
            line_start: 1,
            line_end: body.lines().count().max(1),
            byte_start: 0,
            byte_end: body.len(),
            section_text_lower: section_text.to_lowercase(),
            section_text,
            entities: extracted.note_links,
            attributes: HashMap::new(),
        });
    }
    sections
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_property_drawer_valid() {
        let line = ":ID: arch-v1";
        let result = parse_property_drawer(line);
        assert_eq!(result, Some(("ID".to_string(), "arch-v1".to_string())));
    }

    #[test]
    fn test_parse_property_drawer_with_spaces() {
        let line = "  :TAGS: core, design  ";
        let result = parse_property_drawer(line);
        assert_eq!(
            result,
            Some(("TAGS".to_string(), "core, design".to_string()))
        );
    }

    #[test]
    fn test_parse_property_drawer_no_leading_colon() {
        let line = "ID: arch-v1";
        let result = parse_property_drawer(line);
        assert!(result.is_none());
    }

    #[test]
    fn test_parse_property_drawer_empty_value() {
        let line = ":ID:   ";
        let result = parse_property_drawer(line);
        assert!(result.is_none());
    }

    #[test]
    fn test_extract_property_drawers_multiple() {
        let lines = vec![
            ":ID: test-123".to_string(),
            ":TAGS: one, two".to_string(),
            "".to_string(),
            "Content starts here".to_string(),
        ];
        let attrs = extract_property_drawers(&lines);
        assert_eq!(attrs.get("ID"), Some(&"test-123".to_string()));
        assert_eq!(attrs.get("TAGS"), Some(&"one, two".to_string()));
    }

    #[test]
    fn test_extract_property_drawers_stops_at_content() {
        let lines = vec![
            ":ID: test-456".to_string(),
            "Not a property".to_string(),
            ":TAGS: ignored".to_string(),
        ];
        let attrs = extract_property_drawers(&lines);
        assert_eq!(attrs.get("ID"), Some(&"test-456".to_string()));
        assert!(attrs.get("TAGS").is_none()); // Should not be extracted
    }

    #[test]
    fn test_extract_sections_with_property_drawer() {
        let body = r#"# Main Title
:ID: main-section
:TAGS: important

Content here.

## Subsection
:ID: sub-001

More content.
"#;
        let sections = extract_sections(
            body.as_ref(),
            std::path::Path::new("test.md"),
            std::path::Path::new("/"),
        );

        // First section should have :ID: main-section
        let first = sections.iter().find(|s| s.heading_title == "Main Title");
        assert!(first.is_some());
        let first = first.unwrap();
        assert_eq!(
            first.attributes.get("ID"),
            Some(&"main-section".to_string())
        );
        assert_eq!(first.attributes.get("TAGS"), Some(&"important".to_string()));

        // Subsection should have :ID: sub-001
        let sub = sections.iter().find(|s| s.heading_title == "Subsection");
        assert!(sub.is_some());
        let sub = sub.unwrap();
        assert_eq!(sub.attributes.get("ID"), Some(&"sub-001".to_string()));
    }
}
