//! Issue collection functions for docs governance.

use std::fs;
use std::path::Path;
use walkdir::WalkDir;

use super::parsing::{
    collect_index_body_links, collect_lines, derive_opaque_doc_id, extract_wikilinks,
    is_opaque_doc_id, parse_footer_block, parse_relations_links_line, parse_top_properties_drawer,
};
use super::rendering::{
    link_target, plan_index_footer_block_insertion, plan_index_relations_block_insertion,
    plan_index_section_link_insertion, render_index_footer_with_values, render_package_docs_index,
    render_section_landing_page, standard_section_specs,
};
use super::scope::{scope_matches, scope_matches_doc};
use super::types::{
    DOC_IDENTITY_PROTOCOL_ISSUE_TYPE, INCOMPLETE_PACKAGE_DOCS_INDEX_FOOTER_BLOCK_ISSUE_TYPE,
    MISSING_PACKAGE_DOCS_INDEX_FOOTER_BLOCK_ISSUE_TYPE, MISSING_PACKAGE_DOCS_INDEX_ISSUE_TYPE,
    MISSING_PACKAGE_DOCS_INDEX_RELATION_LINK_ISSUE_TYPE,
    MISSING_PACKAGE_DOCS_INDEX_RELATIONS_BLOCK_ISSUE_TYPE,
    MISSING_PACKAGE_DOCS_INDEX_SECTION_LINK_ISSUE_TYPE,
    MISSING_PACKAGE_DOCS_SECTION_LANDING_ISSUE_TYPE, MISSING_PACKAGE_DOCS_TREE_ISSUE_TYPE,
    STALE_PACKAGE_DOCS_INDEX_FOOTER_STANDARDS_ISSUE_TYPE,
    STALE_PACKAGE_DOCS_INDEX_RELATION_LINK_ISSUE_TYPE,
};
use crate::zhenfa_router::native::semantic_check::{IssueLocation, SemanticIssue};

/// Collects doc governance issues for a single document.
pub fn collect_doc_governance_issues(doc_path: &str, content: &str) -> Vec<SemanticIssue> {
    if !super::parsing::is_package_local_crate_doc(doc_path) {
        return Vec::new();
    }

    let mut issues = Vec::new();
    let expected_id = derive_opaque_doc_id(doc_path);
    let Some(top_drawer) = parse_top_properties_drawer(content) else {
        return Vec::new();
    };

    if let Some(existing_id) = top_drawer.id_line {
        if !is_opaque_doc_id(existing_id.value) {
            issues.push(SemanticIssue {
                severity: "error".to_string(),
                issue_type: DOC_IDENTITY_PROTOCOL_ISSUE_TYPE.to_string(),
                doc: doc_path.to_string(),
                node_id: doc_path.to_string(),
                message: format!(
                    "Top-level :ID: in package-local crate docs must be an opaque hash-shaped identifier, found '{}'",
                    existing_id.value
                ),
                location: Some(IssueLocation {
                    line: existing_id.line,
                    heading_path: "Document Identity".to_string(),
                    byte_range: Some((existing_id.value_start, existing_id.value_end)),
                }),
                suggestion: Some(expected_id),
                fuzzy_suggestion: None,
            });
        }
    } else {
        issues.push(SemanticIssue {
            severity: "error".to_string(),
            issue_type: DOC_IDENTITY_PROTOCOL_ISSUE_TYPE.to_string(),
            doc: doc_path.to_string(),
            node_id: doc_path.to_string(),
            message: "Top-level :ID: is missing from the package-local crate docs property drawer"
                .to_string(),
            location: Some(IssueLocation {
                line: top_drawer.properties_line + 1,
                heading_path: "Document Identity".to_string(),
                byte_range: Some((top_drawer.insert_offset, top_drawer.insert_offset)),
            }),
            suggestion: Some(format!(":ID: {expected_id}{}", top_drawer.newline)),
            fuzzy_suggestion: None,
        });
    }

    issues.extend(collect_stale_index_footer_standards(doc_path, content));
    issues.extend(collect_stale_index_relation_links(doc_path, content));

    issues
}

/// Collects workspace-wide doc governance issues.
#[allow(clippy::too_many_lines)]
pub fn collect_workspace_doc_governance_issues(
    root: &Path,
    scope: Option<&str>,
) -> Vec<SemanticIssue> {
    let crates_dir = root.join("packages").join("rust").join("crates");
    let Ok(entries) = fs::read_dir(crates_dir) else {
        return Vec::new();
    };

    let mut issues = Vec::new();
    for entry in entries.flatten() {
        let package_dir = entry.path();
        if !is_workspace_crate_dir(&package_dir) {
            continue;
        }

        let docs_dir = package_dir.join("docs");
        let index_path = docs_dir.join("index.md");

        let crate_name = package_dir
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown");

        // First check if docs directory exists
        if !docs_dir.is_dir() {
            if scope_matches(scope, &package_dir, &docs_dir, &index_path) {
                issues.push(SemanticIssue {
                    severity: "warning".to_string(),
                    issue_type: MISSING_PACKAGE_DOCS_TREE_ISSUE_TYPE.to_string(),
                    doc: index_path.to_string_lossy().into_owned(),
                    node_id: crate_name.to_string(),
                    message: format!(
                        "Missing documentation tree for package `{crate_name}`. Expected at `docs/`."
                    ),
                    location: None,
                    suggestion: Some(render_package_docs_index(
                        crate_name,
                        &index_path.to_string_lossy(),
                        &docs_dir,
                    )),
                    fuzzy_suggestion: None,
                });
            }
            continue;
        }

        // Recursively check all markdown files in docs/ for identity issues
        for doc_entry in WalkDir::new(&docs_dir).into_iter().flatten() {
            let path = doc_entry.path();
            if path.is_file() && path.extension().and_then(|ext| ext.to_str()) == Some("md") {
                if !scope_matches_doc(scope, &package_dir, &docs_dir, path) {
                    continue;
                }
                if let Ok(content) = fs::read_to_string(path) {
                    issues.extend(collect_doc_governance_issues(
                        &path.to_string_lossy(),
                        &content,
                    ));
                }
            }
        }

        if !scope_matches(scope, &package_dir, &docs_dir, &index_path) {
            continue;
        }

        if !index_path.is_file() {
            issues.push(SemanticIssue {
                severity: "error".to_string(),
                issue_type: MISSING_PACKAGE_DOCS_INDEX_ISSUE_TYPE.to_string(),
                doc: index_path.to_string_lossy().into_owned(),
                node_id: crate_name.to_string(),
                message: format!(
                    "Missing documentation index for package `{crate_name}`. Expected at `docs/index.md`."
                ),
                location: None,
                suggestion: Some(render_package_docs_index(
                    crate_name,
                    &index_path.to_string_lossy(),
                    &docs_dir,
                )),
                fuzzy_suggestion: None,
            });
            continue;
        }

        let Ok(index_content) = fs::read_to_string(&index_path) else {
            continue;
        };

        let index_lines = collect_lines(&index_content);

        // 2. Check index footer block existence
        if parse_footer_block(&index_lines).is_none() {
            let (location, suggestion) = plan_index_footer_block_insertion(&index_content);
            issues.push(SemanticIssue {
                severity: "warning".to_string(),
                issue_type: MISSING_PACKAGE_DOCS_INDEX_FOOTER_BLOCK_ISSUE_TYPE.to_string(),
                doc: index_path.to_string_lossy().into_owned(),
                node_id: crate_name.to_string(),
                message: "Missing mandatory :FOOTER: block in documentation index".to_string(),
                location: Some(location),
                suggestion: Some(suggestion),
                fuzzy_suggestion: None,
            });
        }

        // 3. Check relations block
        let relations_links = parse_relations_links_line(&index_lines);
        let body_links = collect_index_body_links(&index_lines);

        if !body_links.is_empty() {
            match relations_links {
                None => {
                    let (location, suggestion) =
                        plan_index_relations_block_insertion(&index_content, &body_links);
                    issues.push(SemanticIssue {
                        severity: "warning".to_string(),
                        issue_type: MISSING_PACKAGE_DOCS_INDEX_RELATIONS_BLOCK_ISSUE_TYPE.to_string(),
                        doc: index_path.to_string_lossy().into_owned(),
                        node_id: crate_name.to_string(),
                        message: format!("Missing mandatory :RELATIONS: block in documentation index with body links: {}",
                            body_links.iter().map(|l| format!("[[{l}]]")).collect::<Vec<_>>().join(", ")
                        ),
                        location: Some(location),
                        suggestion: Some(suggestion),
                        fuzzy_suggestion: None,
                    });
                }
                Some(links) => {
                    let mut missing_in_relations = Vec::new();
                    for body_link in &body_links {
                        if !links.value.contains(&format!("[[{body_link}]]")) {
                            missing_in_relations.push(body_link.clone());
                        }
                    }

                    if !missing_in_relations.is_empty() {
                        issues.push(SemanticIssue {
                            severity: "warning".to_string(),
                            issue_type: MISSING_PACKAGE_DOCS_INDEX_RELATION_LINK_ISSUE_TYPE
                                .to_string(),
                            doc: index_path.to_string_lossy().into_owned(),
                            node_id: crate_name.to_string(),
                            message: format!(
                                "Documentation links missing from :RELATIONS: block: {}",
                                missing_in_relations
                                    .iter()
                                    .map(|l| format!("[[{l}]]"))
                                    .collect::<Vec<_>>()
                                    .join(", ")
                            ),
                            location: Some(IssueLocation {
                                line: links.line,
                                heading_path: "Index Relations".to_string(),
                                byte_range: Some((links.value_start, links.value_end)),
                            }),
                            suggestion: Some(
                                body_links
                                    .iter()
                                    .map(|l| format!("[[{l}]]"))
                                    .collect::<Vec<_>>()
                                    .join(", "),
                            ),
                            fuzzy_suggestion: None,
                        });
                    }
                }
            }
        }

        // 4. Check section specs and section docs
        let specs = standard_section_specs(crate_name);
        for spec in &specs {
            let section_dir = docs_dir.join(spec.section_name);
            let section_path = docs_dir.join(&spec.relative_path);

            if !scope_matches_doc(scope, &package_dir, &docs_dir, &section_path) {
                continue;
            }

            if !section_path.is_file() {
                issues.push(SemanticIssue {
                    severity: "warning".to_string(),
                    issue_type: MISSING_PACKAGE_DOCS_SECTION_LANDING_ISSUE_TYPE.to_string(),
                    doc: section_path.to_string_lossy().into_owned(),
                    node_id: crate_name.to_string(),
                    message: format!(
                        "Missing mandatory section landing page for `{}` quadrant.",
                        spec.section_name
                    ),
                    location: None,
                    suggestion: Some(render_section_landing_page(
                        crate_name,
                        &package_dir,
                        &section_path.to_string_lossy(),
                        spec,
                    )),
                    fuzzy_suggestion: None,
                });
            }

            // Check if section is linked in index
            let target = link_target(&spec.relative_path);
            if !body_links.iter().any(|l| l == &target) {
                let (location, suggestion) =
                    plan_index_section_link_insertion(&index_content, spec, &target);
                issues.push(SemanticIssue {
                    severity: "warning".to_string(),
                    issue_type: MISSING_PACKAGE_DOCS_INDEX_SECTION_LINK_ISSUE_TYPE.to_string(),
                    doc: index_path.to_string_lossy().into_owned(),
                    node_id: crate_name.to_string(),
                    message: format!(
                        "Mandatory section `{}` is not linked in documentation index.",
                        spec.section_name
                    ),
                    location: Some(location),
                    suggestion: Some(suggestion),
                    fuzzy_suggestion: None,
                });
            }

            // Check directory structure
            if !section_dir.is_dir() {
                issues.push(SemanticIssue {
                    severity: "warning".to_string(),
                    issue_type: MISSING_PACKAGE_DOCS_TREE_ISSUE_TYPE.to_string(),
                    doc: section_dir.to_string_lossy().into_owned(),
                    node_id: crate_name.to_string(),
                    message: format!(
                        "Missing directory tree for `{}` documentation quadrant.",
                        spec.section_name
                    ),
                    location: None,
                    suggestion: None,
                    fuzzy_suggestion: None,
                });
            }
        }
    }

    issues
}

pub fn collect_stale_index_footer_standards(doc_path: &str, content: &str) -> Vec<SemanticIssue> {
    let lines = collect_lines(content);
    let mut issues = Vec::new();

    if let Some(footer) = parse_footer_block(&lines) {
        if footer.standards_value.is_none() || footer.last_sync_value.is_none() {
            issues.push(SemanticIssue {
                severity: "warning".to_string(),
                issue_type: INCOMPLETE_PACKAGE_DOCS_INDEX_FOOTER_BLOCK_ISSUE_TYPE.to_string(),
                doc: doc_path.to_string(),
                node_id: doc_path.to_string(),
                message: "Incomplete :FOOTER: block in documentation index (missing :STANDARDS: or :LAST_SYNC:)".to_string(),
                location: Some(IssueLocation {
                    line: footer.line,
                    heading_path: "Index Footer".to_string(),
                    byte_range: Some((footer.start_offset, footer.end_offset)),
                }),
                suggestion: Some(render_index_footer_with_values(
                    "v2.0",
                    footer.last_sync_value.unwrap_or("pending"),
                )),
                fuzzy_suggestion: None,
            });
        } else if footer.standards_value != Some("v2.0") {
            issues.push(SemanticIssue {
                severity: "warning".to_string(),
                issue_type: STALE_PACKAGE_DOCS_INDEX_FOOTER_STANDARDS_ISSUE_TYPE.to_string(),
                doc: doc_path.to_string(),
                node_id: doc_path.to_string(),
                message: format!(
                    "Stale documentation standards version in index: found '{:?}', expected 'v2.0'",
                    footer.standards_value
                ),
                location: Some(IssueLocation {
                    line: footer.line,
                    heading_path: "Index Footer".to_string(),
                    byte_range: Some((footer.start_offset, footer.end_offset)),
                }),
                suggestion: Some(render_index_footer_with_values(
                    "v2.0",
                    footer.last_sync_value.unwrap_or("pending"),
                )),
                fuzzy_suggestion: None,
            });
        }
    }

    issues
}

pub fn collect_stale_index_relation_links(doc_path: &str, content: &str) -> Vec<SemanticIssue> {
    let lines = collect_lines(content);
    let mut issues = Vec::new();

    if let Some(links_line) = parse_relations_links_line(&lines) {
        let links_in_relations = extract_wikilinks(links_line.value);
        let links_in_body = collect_index_body_links(&lines);

        let stale_links = links_in_relations
            .iter()
            .filter(|l| !links_in_body.contains(l))
            .cloned()
            .collect::<Vec<_>>();

        if !stale_links.is_empty() {
            issues.push(SemanticIssue {
                severity: "warning".to_string(),
                issue_type: STALE_PACKAGE_DOCS_INDEX_RELATION_LINK_ISSUE_TYPE.to_string(),
                doc: doc_path.to_string(),
                node_id: doc_path.to_string(),
                message: format!(
                    "Documentation links in :RELATIONS: block are no longer present in body: {}",
                    stale_links
                        .iter()
                        .map(|l| format!("[[{l}]]"))
                        .collect::<Vec<_>>()
                        .join(", ")
                ),
                location: Some(IssueLocation {
                    line: links_line.line,
                    heading_path: "Index Relations".to_string(),
                    byte_range: Some((links_line.value_start, links_line.value_end)),
                }),
                suggestion: Some(
                    links_in_body
                        .iter()
                        .map(|l| format!("[[{l}]]"))
                        .collect::<Vec<_>>()
                        .join(", "),
                ),
                fuzzy_suggestion: None,
            });
        }
    }

    issues
}

fn is_workspace_crate_dir(path: &Path) -> bool {
    path.join("Cargo.toml").is_file()
}
