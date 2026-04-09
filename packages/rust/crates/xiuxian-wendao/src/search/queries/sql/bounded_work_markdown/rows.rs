use std::collections::HashMap;
use std::fs;
use std::path::Path;

use crate::parsers::markdown::parse_note;

use super::discovery::DiscoveredMarkdownFile;
use super::skeleton::render_markdown_skeleton;

/// One queryable structural markdown unit inside a bounded work surface.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BoundedWorkMarkdownRow {
    /// The markdown file path relative to the bounded-work root.
    pub path: String,
    /// The top-level bounded-work surface that owns this row.
    pub surface: String,
    /// The normalized heading path using `/` as the segment separator.
    pub heading_path: String,
    /// The current row title or effective section title.
    pub title: String,
    /// The heading level for this row, or `0` for the document root row.
    pub level: i64,
    /// The structural compression view used for low-token reads.
    pub skeleton: String,
    /// The full markdown body for this structural unit.
    pub body: String,
}

pub(crate) fn build_rows_for_file(
    root: &Path,
    file: &DiscoveredMarkdownFile,
) -> Result<Vec<BoundedWorkMarkdownRow>, String> {
    let body = fs::read_to_string(&file.absolute_path).map_err(|error| {
        format!(
            "failed to read bounded work markdown file `{}`: {error}",
            file.absolute_path.display()
        )
    })?;
    let parsed = parse_note(&file.absolute_path, root, &body).ok_or_else(|| {
        format!(
            "failed to parse bounded work markdown file `{}`",
            file.absolute_path.display()
        )
    })?;

    let mut rows = Vec::with_capacity(parsed.sections.len() + 1);
    rows.push(BoundedWorkMarkdownRow {
        path: file.relative_path.clone(),
        surface: file.surface.clone(),
        heading_path: String::new(),
        title: parsed.doc.title.clone(),
        level: 0,
        skeleton: render_markdown_skeleton(&parsed.doc.title, 1, &HashMap::new(), &body),
        body: body.trim().to_string(),
    });

    rows.extend(parsed.sections.iter().map(|section| {
        let heading_path = normalize_heading_path(section.heading_path.as_str());
        let title = effective_title(heading_path.as_str(), section.heading_title.as_str());
        BoundedWorkMarkdownRow {
            path: file.relative_path.clone(),
            surface: file.surface.clone(),
            heading_path,
            title: title.clone(),
            level: i64::try_from(section.heading_level).unwrap_or(i64::MAX),
            skeleton: render_markdown_skeleton(
                title.as_str(),
                section.heading_level.max(1),
                &section.attributes,
                section.section_text.as_str(),
            ),
            body: section.section_text.trim().to_string(),
        }
    }));

    Ok(rows)
}

fn normalize_heading_path(raw: &str) -> String {
    raw.split(" / ")
        .map(str::trim)
        .filter(|segment| !segment.is_empty())
        .collect::<Vec<_>>()
        .join("/")
}

fn effective_title(heading_path: &str, heading_title: &str) -> String {
    if !heading_title.trim().is_empty() {
        return heading_title.trim().to_string();
    }
    heading_path
        .rsplit('/')
        .next()
        .map(str::trim)
        .filter(|segment| !segment.is_empty())
        .unwrap_or("Overview")
        .to_string()
}
