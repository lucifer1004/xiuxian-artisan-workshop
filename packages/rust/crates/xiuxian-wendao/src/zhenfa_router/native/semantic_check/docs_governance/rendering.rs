//! Template rendering utilities for docs governance.

use std::fmt::Write as _;
use std::fs;
use std::path::Path;

use super::parsing::derive_opaque_doc_id;
use super::types::SectionSpec;
use crate::zhenfa_router::native::semantic_check::IssueLocation;

/// Renders a package docs index template.
pub fn render_package_docs_index(crate_name: &str, doc_path: &str, docs_dir: &Path) -> String {
    let section_links = collect_section_links(docs_dir);
    let mut rendered = String::new();

    let _ = writeln!(rendered, "# {crate_name}: Map of Content");
    rendered.push('\n');
    rendered.push_str(":PROPERTIES:\n");
    let _ = writeln!(rendered, ":ID: {}", derive_opaque_doc_id(doc_path));
    rendered.push_str(":TYPE: INDEX\n");
    rendered.push_str(":STATUS: ACTIVE\n");
    rendered.push_str(":END:\n\n");
    let _ = writeln!(
        rendered,
        "Standardized documentation index for the `{crate_name}` package.\n"
    );

    if section_links.is_empty() {
        rendered.push_str(
            "Populate package-local documentation sections under this directory and extend this index as the package surface evolves.\n",
        );
        rendered.push_str("\n---\n\n");
        rendered.push_str(&render_index_footer());
        return rendered;
    }

    for (section, links) in &section_links {
        let _ = writeln!(rendered, "## {section}\n");
        for link in links {
            let _ = writeln!(rendered, "- [[{link}]]");
        }
        rendered.push('\n');
    }

    rendered.push_str(":RELATIONS:\n");
    rendered.push_str(":LINKS: ");
    rendered.push_str(
        &section_links
            .iter()
            .flat_map(|(_, links)| links.iter())
            .map(|link| format!("[[{link}]]"))
            .collect::<Vec<_>>()
            .join(", "),
    );
    rendered.push_str("\n:END:\n");
    rendered.push_str("\n---\n\n");
    rendered.push_str(&render_index_footer());
    rendered
}

/// Renders a section landing page template.
pub fn render_section_landing_page(
    crate_name: &str,
    crate_dir: &Path,
    doc_path: &str,
    spec: &SectionSpec,
) -> String {
    let mut rendered = String::new();
    let _ = writeln!(rendered, "# {}\n", spec.title);
    rendered.push_str(":PROPERTIES:\n");
    let _ = writeln!(rendered, ":ID: {}", derive_opaque_doc_id(doc_path));
    let _ = writeln!(rendered, ":TYPE: {}", spec.doc_type);
    rendered.push_str(":STATUS: DRAFT\n");
    rendered.push_str(":END:\n\n");
    let _ = writeln!(
        rendered,
        "{}\n",
        render_section_summary(crate_name, crate_dir, spec)
    );
    let _ = writeln!(rendered, "{}", render_section_prompt(crate_name, spec));
    rendered
}

fn render_index_footer() -> String {
    render_index_footer_with_values("v2.0", "pending")
}

pub fn render_index_footer_with_values(standards: &str, last_sync: &str) -> String {
    format!(":FOOTER:\n:STANDARDS: {standards}\n:LAST_SYNC: {last_sync}\n:END:\n")
}

pub fn link_target(relative_path: &str) -> String {
    relative_path
        .strip_suffix(".md")
        .unwrap_or(relative_path)
        .replace('\\', "/")
}

fn collect_section_links(docs_dir: &Path) -> Vec<(String, Vec<String>)> {
    let Ok(entries) = fs::read_dir(docs_dir) else {
        return Vec::new();
    };

    let mut section_links = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }

        let Some(section_name) = path.file_name().and_then(|name| name.to_str()) else {
            continue;
        };

        let Ok(section_entries) = fs::read_dir(&path) else {
            continue;
        };

        let mut links = section_entries
            .flatten()
            .filter_map(|child| {
                let child_path = child.path();
                if !child_path.is_file() {
                    return None;
                }
                if child_path.extension().and_then(|ext| ext.to_str()) != Some("md") {
                    return None;
                }
                let stem = child_path.file_stem()?.to_str()?;
                Some(format!("{section_name}/{stem}"))
            })
            .collect::<Vec<_>>();
        links.sort();

        if !links.is_empty() {
            section_links.push((section_name.to_string(), links));
        }
    }

    section_links.sort_by(|left, right| left.0.cmp(&right.0));
    section_links
}

pub fn standard_section_specs(crate_name: &str) -> Vec<SectionSpec> {
    let slug = crate_slug(crate_name);
    vec![
        SectionSpec {
            section_name: "01_core",
            relative_path: format!("01_core/101_{slug}_core_boundary.md"),
            title: "Core Boundary".to_string(),
            doc_type: "CORE",
        },
        SectionSpec {
            section_name: "03_features",
            relative_path: format!("03_features/201_{slug}_feature_ledger.md"),
            title: "Feature Ledger".to_string(),
            doc_type: "FEATURE",
        },
        SectionSpec {
            section_name: "05_research",
            relative_path: format!("05_research/301_{slug}_research_agenda.md"),
            title: "Research Agenda".to_string(),
            doc_type: "RESEARCH",
        },
        SectionSpec {
            section_name: "06_roadmap",
            relative_path: format!("06_roadmap/401_{slug}_roadmap.md"),
            title: "Roadmap".to_string(),
            doc_type: "ROADMAP",
        },
    ]
}

fn crate_slug(crate_name: &str) -> String {
    crate_name
        .chars()
        .map(|ch| if ch.is_ascii_alphanumeric() { ch } else { '_' })
        .collect()
}

fn render_section_summary(crate_name: &str, crate_dir: &Path, spec: &SectionSpec) -> String {
    let crate_kind = if crate_dir.join("src/lib.rs").is_file() {
        "library crate"
    } else if crate_dir.join("src/main.rs").is_file() {
        "binary crate"
    } else {
        "Rust crate"
    };

    match spec.section_name {
        "01_core" => format!(
            "Architecture boundary note for the `{crate_name}` {crate_kind}. Capture core responsibilities, integration edges, and invariants here."
        ),
        "03_features" => format!(
            "Feature ledger for the `{crate_name}` {crate_kind}. Track user-facing or system-facing capabilities implemented in this package."
        ),
        "05_research" => format!(
            "Research agenda for the `{crate_name}` {crate_kind}. Record external references, experiments, and design questions that still need hardening."
        ),
        "06_roadmap" => format!(
            "Roadmap tracker for the `{crate_name}` {crate_kind}. Use this page to pin the next implementation milestones and validation gates."
        ),
        _ => format!("Documentation placeholder for `{crate_name}`."),
    }
}

fn render_section_prompt(crate_name: &str, spec: &SectionSpec) -> String {
    match spec.section_name {
        "01_core" => format!(
            "Document the stable architectural boundary for `{crate_name}` before expanding deeper feature notes."
        ),
        "03_features" => format!(
            "Promote concrete `{crate_name}` capabilities into this ledger as feature slices land."
        ),
        "05_research" => format!(
            "Capture unresolved research questions and external references that inform `{crate_name}`."
        ),
        "06_roadmap" => format!(
            "List the next verified milestones for `{crate_name}` and keep them synchronized with GTD and ExecPlans."
        ),
        _ => "Extend this placeholder with package-specific detail.".to_string(),
    }
}

pub fn plan_index_relations_block_insertion(
    index_content: &str,
    body_links: &[String],
) -> (IssueLocation, String) {
    let lines = super::parsing::collect_lines(index_content);
    let insertion_line = lines
        .iter()
        .find(|line| line.trimmed == "---" || line.trimmed == ":FOOTER:");
    let insert_offset = insertion_line.map_or(index_content.len(), |line| line.start_offset);
    let prefix = if insert_offset == 0 || index_content[..insert_offset].ends_with("\n\n") {
        ""
    } else if index_content[..insert_offset].ends_with('\n') {
        "\n"
    } else {
        "\n\n"
    };
    let suffix = if insertion_line.is_some() { "\n" } else { "" };

    (
        IssueLocation {
            line: insertion_line
                .or_else(|| lines.last())
                .map_or(1, |line| line.line_number),
            heading_path: "Index Relations".to_string(),
            byte_range: Some((insert_offset, insert_offset)),
        },
        format!(
            "{prefix}:RELATIONS:\n:LINKS: {}\n:END:\n{suffix}",
            body_links
                .iter()
                .map(|link| format!("[[{link}]]"))
                .collect::<Vec<_>>()
                .join(", ")
        ),
    )
}

pub fn plan_index_footer_block_insertion(index_content: &str) -> (IssueLocation, String) {
    let lines = super::parsing::collect_lines(index_content);
    let insert_offset = index_content.len();
    let prefix = if index_content.is_empty() || index_content.ends_with("\n\n") {
        ""
    } else if index_content.ends_with('\n') {
        "\n"
    } else {
        "\n\n"
    };

    (
        IssueLocation {
            line: lines.last().map_or(1, |line| line.line_number),
            heading_path: "Index Footer".to_string(),
            byte_range: Some((insert_offset, insert_offset)),
        },
        format!("{prefix}---\n\n{}", render_index_footer()),
    )
}

pub fn plan_index_section_link_insertion(
    index_content: &str,
    spec: &SectionSpec,
    link_target: &str,
) -> (IssueLocation, String) {
    let lines = super::parsing::collect_lines(index_content);

    if let Some((heading_idx, heading_line)) = lines
        .iter()
        .enumerate()
        .find(|(_, line)| matches_section_heading(line.trimmed, spec.section_name))
    {
        let next_heading_idx = lines
            .iter()
            .enumerate()
            .skip(heading_idx + 1)
            .find(|(_, line)| line.trimmed.starts_with("## "))
            .map_or(lines.len(), |(idx, _)| idx);

        let section_lines = &lines[heading_idx + 1..next_heading_idx];
        if let Some(anchor) = section_lines
            .iter()
            .rev()
            .find(|line| !line.trimmed.is_empty())
        {
            let prefix = if anchor.newline.is_empty() { "\n" } else { "" };
            return (
                IssueLocation {
                    line: anchor.line_number,
                    heading_path: spec.section_name.to_string(),
                    byte_range: Some((anchor.end_offset, anchor.end_offset)),
                },
                format!("{prefix}- [[{link_target}]]\n"),
            );
        }

        let insert_offset = section_lines
            .iter()
            .take_while(|line| line.trimmed.is_empty())
            .last()
            .map_or(heading_line.end_offset, |line| line.end_offset);
        let prefix = if insert_offset == heading_line.end_offset {
            "\n"
        } else {
            ""
        };
        return (
            IssueLocation {
                line: heading_line.line_number,
                heading_path: spec.section_name.to_string(),
                byte_range: Some((insert_offset, insert_offset)),
            },
            format!("{prefix}- [[{link_target}]]\n"),
        );
    }

    let insertion_line = lines.iter().find(|line| {
        line.trimmed == ":RELATIONS:" || line.trimmed == "---" || line.trimmed == ":FOOTER:"
    });
    let insert_offset = insertion_line.map_or(index_content.len(), |line| line.start_offset);
    let prefix = if index_content.is_empty()
        || (insert_offset > 0 && index_content[..insert_offset].ends_with("\n\n"))
    {
        ""
    } else if insert_offset > 0 && index_content[..insert_offset].ends_with('\n') {
        "\n"
    } else {
        "\n\n"
    };
    let suffix = if insertion_line.is_some() { "\n" } else { "" };

    (
        IssueLocation {
            line: insertion_line
                .or_else(|| lines.last())
                .map_or(1, |line| line.line_number),
            heading_path: "Docs Index".to_string(),
            byte_range: Some((insert_offset, insert_offset)),
        },
        format!(
            "{prefix}## {}\n\n- [[{link_target}]]\n{suffix}",
            spec.section_name
        ),
    )
}

fn matches_section_heading(trimmed: &str, section_name: &str) -> bool {
    let heading = format!("## {section_name}");
    trimmed == heading || trimmed.starts_with(&format!("{heading}:"))
}
