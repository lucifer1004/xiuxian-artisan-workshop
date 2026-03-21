use std::fs;
use std::path::{Component, Path, PathBuf};

use sha1::{Digest, Sha1};

use super::{IssueLocation, SemanticIssue};

pub(crate) const DOC_IDENTITY_PROTOCOL_ISSUE_TYPE: &str = "doc_identity_protocol";
pub(crate) const MISSING_PACKAGE_DOCS_TREE_ISSUE_TYPE: &str = "missing_package_docs_tree";
pub(crate) const MISSING_PACKAGE_DOCS_INDEX_ISSUE_TYPE: &str = "missing_package_docs_index";
pub(crate) const MISSING_PACKAGE_DOCS_SECTION_LANDING_ISSUE_TYPE: &str =
    "missing_package_docs_section_landing";
pub(crate) const MISSING_PACKAGE_DOCS_INDEX_SECTION_LINK_ISSUE_TYPE: &str =
    "missing_package_docs_index_section_link";
pub(crate) const MISSING_PACKAGE_DOCS_INDEX_RELATIONS_BLOCK_ISSUE_TYPE: &str =
    "missing_package_docs_index_relations_block";
pub(crate) const MISSING_PACKAGE_DOCS_INDEX_RELATION_LINK_ISSUE_TYPE: &str =
    "missing_package_docs_index_relation_link";
pub(crate) const STALE_PACKAGE_DOCS_INDEX_RELATION_LINK_ISSUE_TYPE: &str =
    "stale_package_docs_index_relation_link";
pub(crate) const MISSING_PACKAGE_DOCS_INDEX_FOOTER_BLOCK_ISSUE_TYPE: &str =
    "missing_package_docs_index_footer_block";
pub(crate) const INCOMPLETE_PACKAGE_DOCS_INDEX_FOOTER_BLOCK_ISSUE_TYPE: &str =
    "incomplete_package_docs_index_footer_block";
pub(crate) const STALE_PACKAGE_DOCS_INDEX_FOOTER_STANDARDS_ISSUE_TYPE: &str =
    "stale_package_docs_index_footer_standards";

pub(crate) fn collect_doc_governance_issues(doc_path: &str, content: &str) -> Vec<SemanticIssue> {
    if !is_package_local_crate_doc(doc_path) {
        return Vec::new();
    }

    let expected_id = derive_opaque_doc_id(doc_path);
    let Some(top_drawer) = parse_top_properties_drawer(content) else {
        return Vec::new();
    };

    if let Some(existing_id) = top_drawer.id_line {
        if is_opaque_doc_id(existing_id.value) {
            return Vec::new();
        }

        return vec![SemanticIssue {
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
        }];
    }

    vec![SemanticIssue {
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
    }]
}

pub(crate) fn collect_workspace_doc_governance_issues(
    root: &Path,
    scope: Option<&str>,
) -> Vec<SemanticIssue> {
    let crates_dir = root.join("packages").join("rust").join("crates");
    let Ok(entries) = fs::read_dir(crates_dir) else {
        return Vec::new();
    };

    let mut issues = Vec::new();
    for entry in entries.flatten() {
        let crate_dir = entry.path();
        if !is_workspace_crate_dir(&crate_dir) {
            continue;
        }

        let docs_dir = crate_dir.join("docs");
        let index_path = docs_dir.join("index.md");
        if !scope_matches(scope, &crate_dir, &docs_dir, &index_path) {
            continue;
        }

        let crate_name = crate_dir
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("crate");

        if !docs_dir.is_dir() {
            let doc_path = index_path.to_string_lossy().into_owned();
            issues.push(SemanticIssue {
                severity: "warning".to_string(),
                issue_type: MISSING_PACKAGE_DOCS_TREE_ISSUE_TYPE.to_string(),
                doc: doc_path.clone(),
                node_id: doc_path.clone(),
                message: format!(
                    "Package-local crate docs tree is missing for crate '{crate_name}'"
                ),
                location: Some(IssueLocation {
                    line: 1,
                    heading_path: "Docs Bootstrap".to_string(),
                    byte_range: None,
                }),
                suggestion: Some(render_package_docs_index(crate_name, &doc_path, &docs_dir)),
                fuzzy_suggestion: None,
            });
            continue;
        }

        issues.extend(collect_package_doc_identity_issues(
            &crate_dir, &docs_dir, scope,
        ));

        if index_path.is_file() {
            issues.extend(collect_missing_index_footer_block(&index_path));
            issues.extend(collect_incomplete_index_footer_block(&index_path));
            issues.extend(collect_stale_index_footer_standards(&index_path));
            issues.extend(collect_missing_index_relations_block(&index_path));
            issues.extend(collect_stale_index_relation_links(&index_path));
            issues.extend(collect_missing_index_relation_links(&index_path));
            issues.extend(collect_missing_index_section_links(
                crate_name,
                &index_path,
                &docs_dir,
            ));
            issues.extend(collect_missing_section_landings(
                crate_name, &crate_dir, &docs_dir,
            ));
            continue;
        }

        let doc_path = index_path.to_string_lossy().into_owned();
        issues.push(SemanticIssue {
            severity: "error".to_string(),
            issue_type: MISSING_PACKAGE_DOCS_INDEX_ISSUE_TYPE.to_string(),
            doc: doc_path.clone(),
            node_id: doc_path.clone(),
            message: format!(
                "Package-local crate docs directory is missing required index.md for crate '{crate_name}'"
            ),
            location: Some(IssueLocation {
                line: 1,
                heading_path: "Docs Index".to_string(),
                byte_range: None,
            }),
            suggestion: Some(render_package_docs_index(crate_name, &doc_path, &docs_dir)),
            fuzzy_suggestion: None,
        });
    }

    issues
}

fn collect_package_doc_identity_issues(
    crate_dir: &Path,
    docs_dir: &Path,
    scope: Option<&str>,
) -> Vec<SemanticIssue> {
    collect_package_doc_paths(docs_dir)
        .into_iter()
        .filter(|doc_path| scope_matches_doc(scope, crate_dir, docs_dir, doc_path))
        .flat_map(|doc_path| {
            let Ok(content) = fs::read_to_string(&doc_path) else {
                return Vec::new();
            };
            collect_doc_governance_issues(&doc_path.to_string_lossy(), &content)
        })
        .collect()
}

pub(crate) fn derive_opaque_doc_id(doc_path: &str) -> String {
    let mut hasher = Sha1::new();
    hasher.update(normalize_doc_path(doc_path).as_bytes());
    format!("{:x}", hasher.finalize())
}

pub(crate) fn is_opaque_doc_id(value: &str) -> bool {
    value.len() == 40
        && value
            .chars()
            .all(|ch| ch.is_ascii_hexdigit() && !ch.is_ascii_uppercase())
}

fn normalize_doc_path(doc_path: &str) -> String {
    let normalized = doc_path.replace('\\', "/");
    normalized
        .find("packages/rust/crates/")
        .map_or(normalized.clone(), |idx| normalized[idx..].to_string())
}

fn is_workspace_crate_dir(crate_dir: &Path) -> bool {
    crate_dir.is_dir() && crate_dir.join("Cargo.toml").is_file()
}

fn collect_missing_section_landings(
    crate_name: &str,
    crate_dir: &Path,
    docs_dir: &Path,
) -> Vec<SemanticIssue> {
    standard_section_specs(crate_name)
        .into_iter()
        .filter_map(|spec| {
            let doc_path = docs_dir.join(&spec.relative_path);
            if doc_path.is_file() {
                return None;
            }

            let doc_path_str = doc_path.to_string_lossy().into_owned();
            Some(SemanticIssue {
                severity: "warning".to_string(),
                issue_type: MISSING_PACKAGE_DOCS_SECTION_LANDING_ISSUE_TYPE.to_string(),
                doc: doc_path_str.clone(),
                node_id: doc_path_str.clone(),
                message: format!(
                    "Package-local docs are missing the standard {} section landing page for crate '{}'",
                    spec.section_name, crate_name
                ),
                location: Some(IssueLocation {
                    line: 1,
                    heading_path: spec.title.to_string(),
                    byte_range: None,
                }),
                suggestion: Some(render_section_landing_page(crate_name, crate_dir, &doc_path_str, &spec)),
                fuzzy_suggestion: None,
            })
        })
        .collect()
}

fn collect_missing_index_relations_block(index_path: &Path) -> Vec<SemanticIssue> {
    let Ok(index_content) = fs::read_to_string(index_path) else {
        return Vec::new();
    };
    let lines = collect_lines(&index_content);
    let body_links = collect_index_body_links(&lines);
    if body_links.is_empty() || parse_relations_links_line(&lines).is_some() {
        return Vec::new();
    }

    let (location, suggestion) = plan_index_relations_block_insertion(&index_content, &body_links);
    let index_path_str = index_path.to_string_lossy().into_owned();
    vec![SemanticIssue {
        severity: "warning".to_string(),
        issue_type: MISSING_PACKAGE_DOCS_INDEX_RELATIONS_BLOCK_ISSUE_TYPE.to_string(),
        doc: index_path_str.clone(),
        node_id: index_path_str.clone(),
        message: format!(
            "Package-local docs index is missing the :RELATIONS: :LINKS: block for body links: {}",
            body_links
                .iter()
                .map(|link| format!("[[{link}]]"))
                .collect::<Vec<_>>()
                .join(", ")
        ),
        location: Some(location),
        suggestion: Some(suggestion),
        fuzzy_suggestion: None,
    }]
}

fn collect_missing_index_footer_block(index_path: &Path) -> Vec<SemanticIssue> {
    let Ok(index_content) = fs::read_to_string(index_path) else {
        return Vec::new();
    };
    let lines = collect_lines(&index_content);
    if lines.iter().any(|line| line.trimmed == ":FOOTER:")
        || parse_relations_links_line(&lines).is_none()
    {
        return Vec::new();
    }

    let (location, suggestion) = plan_index_footer_block_insertion(&index_content);
    let index_path_str = index_path.to_string_lossy().into_owned();
    vec![SemanticIssue {
        severity: "warning".to_string(),
        issue_type: MISSING_PACKAGE_DOCS_INDEX_FOOTER_BLOCK_ISSUE_TYPE.to_string(),
        doc: index_path_str.clone(),
        node_id: index_path_str.clone(),
        message:
            "Package-local docs index is missing the :FOOTER: block after the relations section"
                .to_string(),
        location: Some(location),
        suggestion: Some(suggestion),
        fuzzy_suggestion: None,
    }]
}

fn collect_incomplete_index_footer_block(index_path: &Path) -> Vec<SemanticIssue> {
    let Ok(index_content) = fs::read_to_string(index_path) else {
        return Vec::new();
    };
    let lines = collect_lines(&index_content);
    let Some(footer_block) = parse_footer_block(&lines) else {
        return Vec::new();
    };

    let mut missing_fields = Vec::new();
    if footer_block.standards_value.is_none() {
        missing_fields.push(":STANDARDS:");
    }
    if footer_block.last_sync_value.is_none() {
        missing_fields.push(":LAST_SYNC:");
    }
    if missing_fields.is_empty() {
        return Vec::new();
    }

    let index_path_str = index_path.to_string_lossy().into_owned();
    vec![SemanticIssue {
        severity: "warning".to_string(),
        issue_type: INCOMPLETE_PACKAGE_DOCS_INDEX_FOOTER_BLOCK_ISSUE_TYPE.to_string(),
        doc: index_path_str.clone(),
        node_id: index_path_str.clone(),
        message: format!(
            "Package-local docs index footer block is missing required fields: {}",
            missing_fields.join(", ")
        ),
        location: Some(IssueLocation {
            line: footer_block.line,
            heading_path: "Index Footer".to_string(),
            byte_range: Some((footer_block.start_offset, footer_block.end_offset)),
        }),
        suggestion: Some(render_index_footer_with_values(
            footer_block.standards_value.unwrap_or("v2.0"),
            footer_block.last_sync_value.unwrap_or("pending"),
        )),
        fuzzy_suggestion: None,
    }]
}

fn collect_stale_index_footer_standards(index_path: &Path) -> Vec<SemanticIssue> {
    let Ok(index_content) = fs::read_to_string(index_path) else {
        return Vec::new();
    };
    let lines = collect_lines(&index_content);
    let Some(footer_block) = parse_footer_block(&lines) else {
        return Vec::new();
    };

    let Some(standards_value) = footer_block.standards_value else {
        return Vec::new();
    };
    if footer_block.last_sync_value.is_none() || standards_value == "v2.0" {
        return Vec::new();
    }

    let index_path_str = index_path.to_string_lossy().into_owned();
    vec![SemanticIssue {
        severity: "warning".to_string(),
        issue_type: STALE_PACKAGE_DOCS_INDEX_FOOTER_STANDARDS_ISSUE_TYPE.to_string(),
        doc: index_path_str.clone(),
        node_id: index_path_str.clone(),
        message: format!(
            "Package-local docs index footer block uses stale :STANDARDS: value '{standards_value}', expected 'v2.0'"
        ),
        location: Some(IssueLocation {
            line: footer_block.line,
            heading_path: "Index Footer".to_string(),
            byte_range: Some((footer_block.start_offset, footer_block.end_offset)),
        }),
        suggestion: Some(render_index_footer_with_values(
            "v2.0",
            footer_block.last_sync_value.unwrap_or("pending"),
        )),
        fuzzy_suggestion: None,
    }]
}

fn collect_missing_index_section_links(
    crate_name: &str,
    index_path: &Path,
    docs_dir: &Path,
) -> Vec<SemanticIssue> {
    let Ok(index_content) = fs::read_to_string(index_path) else {
        return Vec::new();
    };

    standard_section_specs(crate_name)
        .into_iter()
        .filter_map(|spec| {
            let section_doc_path = docs_dir.join(&spec.relative_path);
            if !section_doc_path.is_file() {
                return None;
            }

            let link_target = link_target(&spec.relative_path);
            if index_content.contains(&format!("[[{link_target}]]")) {
                return None;
            }

            let (location, suggestion) =
                plan_index_section_link_insertion(&index_content, &spec, &link_target);
            let index_path_str = index_path.to_string_lossy().into_owned();
            Some(SemanticIssue {
                severity: "warning".to_string(),
                issue_type: MISSING_PACKAGE_DOCS_INDEX_SECTION_LINK_ISSUE_TYPE.to_string(),
                doc: index_path_str.clone(),
                node_id: index_path_str.clone(),
                message: format!(
                    "Package-local docs index is missing the standard {} section link [[{}]] for crate '{}'",
                    spec.section_name, link_target, crate_name
                ),
                location: Some(location),
                suggestion: Some(suggestion),
                fuzzy_suggestion: None,
            })
        })
        .collect()
}

fn collect_missing_index_relation_links(index_path: &Path) -> Vec<SemanticIssue> {
    let Ok(index_content) = fs::read_to_string(index_path) else {
        return Vec::new();
    };
    let lines = collect_lines(&index_content);
    let Some(links_line) = parse_relations_links_line(&lines) else {
        return Vec::new();
    };

    let body_links = collect_index_body_links(&lines);
    if body_links.is_empty() {
        return Vec::new();
    }

    let relation_links = extract_wikilinks(links_line.value);
    let missing_links: Vec<_> = body_links
        .iter()
        .filter(|link| !relation_links.iter().any(|existing| existing == *link))
        .cloned()
        .collect();
    if missing_links.is_empty() {
        return Vec::new();
    }

    let mut merged_links = relation_links;
    for link in &missing_links {
        if !merged_links.iter().any(|existing| existing == link) {
            merged_links.push(link.clone());
        }
    }

    let index_path_str = index_path.to_string_lossy().into_owned();
    vec![SemanticIssue {
        severity: "warning".to_string(),
        issue_type: MISSING_PACKAGE_DOCS_INDEX_RELATION_LINK_ISSUE_TYPE.to_string(),
        doc: index_path_str.clone(),
        node_id: index_path_str.clone(),
        message: format!(
            "Package-local docs index :RELATIONS: :LINKS: block is missing body links: {}",
            missing_links
                .iter()
                .map(|link| format!("[[{link}]]"))
                .collect::<Vec<_>>()
                .join(", ")
        ),
        location: Some(IssueLocation {
            line: links_line.line,
            heading_path: "Index Relations".to_string(),
            byte_range: Some((links_line.value_start, links_line.value_end)),
        }),
        suggestion: Some(
            merged_links
                .iter()
                .map(|link| format!("[[{link}]]"))
                .collect::<Vec<_>>()
                .join(", "),
        ),
        fuzzy_suggestion: None,
    }]
}

fn collect_stale_index_relation_links(index_path: &Path) -> Vec<SemanticIssue> {
    let Ok(index_content) = fs::read_to_string(index_path) else {
        return Vec::new();
    };
    let lines = collect_lines(&index_content);
    let Some(links_line) = parse_relations_links_line(&lines) else {
        return Vec::new();
    };

    let body_links = collect_index_body_links(&lines);
    if body_links.is_empty() {
        return Vec::new();
    }

    let relation_links = extract_wikilinks(links_line.value);
    let missing_links: Vec<_> = body_links
        .iter()
        .filter(|link| !relation_links.iter().any(|existing| existing == *link))
        .cloned()
        .collect();
    if !missing_links.is_empty() {
        return Vec::new();
    }

    let stale_links: Vec<_> = relation_links
        .iter()
        .filter(|link| !body_links.iter().any(|existing| existing == *link))
        .cloned()
        .collect();
    if stale_links.is_empty() {
        return Vec::new();
    }

    let index_path_str = index_path.to_string_lossy().into_owned();
    vec![SemanticIssue {
        severity: "warning".to_string(),
        issue_type: STALE_PACKAGE_DOCS_INDEX_RELATION_LINK_ISSUE_TYPE.to_string(),
        doc: index_path_str.clone(),
        node_id: index_path_str.clone(),
        message: format!(
            "Package-local docs index :RELATIONS: :LINKS: block contains stale links not present in the index body: {}",
            stale_links
                .iter()
                .map(|link| format!("[[{link}]]"))
                .collect::<Vec<_>>()
                .join(", ")
        ),
        location: Some(IssueLocation {
            line: links_line.line,
            heading_path: "Index Relations".to_string(),
            byte_range: Some((links_line.value_start, links_line.value_end)),
        }),
        suggestion: Some(
            body_links
                .iter()
                .map(|link| format!("[[{link}]]"))
                .collect::<Vec<_>>()
                .join(", "),
        ),
        fuzzy_suggestion: None,
    }]
}

fn scope_matches(
    scope: Option<&str>,
    crate_dir: &Path,
    docs_dir: &Path,
    index_path: &Path,
) -> bool {
    let Some(scope) = scope else {
        return true;
    };
    if scope.is_empty() || scope == "." {
        return true;
    }

    if scope_looks_path_like(scope) {
        return path_scope_matches(scope, crate_dir)
            || path_scope_matches(scope, docs_dir)
            || path_scope_matches(scope, index_path);
    }

    let normalized_scope = scope.replace('\\', "/").to_lowercase();
    let crate_path = crate_dir
        .to_string_lossy()
        .replace('\\', "/")
        .to_lowercase();
    let docs_path = docs_dir.to_string_lossy().replace('\\', "/").to_lowercase();
    let index_path = index_path
        .to_string_lossy()
        .replace('\\', "/")
        .to_lowercase();

    crate_path.contains(&normalized_scope)
        || docs_path.contains(&normalized_scope)
        || index_path.contains(&normalized_scope)
        || normalized_scope.contains(&crate_path)
        || normalized_scope.contains(&docs_path)
        || normalized_scope.contains(&index_path)
}

fn scope_matches_doc(
    scope: Option<&str>,
    crate_dir: &Path,
    docs_dir: &Path,
    doc_path: &Path,
) -> bool {
    let Some(scope) = scope else {
        return true;
    };
    if scope.is_empty() || scope == "." {
        return true;
    }

    if scope_looks_path_like(scope) {
        if Path::new(scope).extension().and_then(|ext| ext.to_str()) == Some("md") {
            return path_scope_matches(scope, doc_path);
        }

        return path_scope_matches(scope, crate_dir)
            || path_scope_matches(scope, docs_dir)
            || path_scope_matches(scope, doc_path);
    }

    let normalized_scope = scope.replace('\\', "/").to_lowercase();
    let crate_path = crate_dir
        .to_string_lossy()
        .replace('\\', "/")
        .to_lowercase();
    let docs_path = docs_dir.to_string_lossy().replace('\\', "/").to_lowercase();
    let doc_path = doc_path.to_string_lossy().replace('\\', "/").to_lowercase();

    crate_path.contains(&normalized_scope)
        || docs_path.contains(&normalized_scope)
        || doc_path.contains(&normalized_scope)
        || normalized_scope.contains(&crate_path)
        || normalized_scope.contains(&docs_path)
        || normalized_scope.contains(&doc_path)
}

fn scope_looks_path_like(scope: &str) -> bool {
    scope.contains('/')
        || scope.contains('\\')
        || scope.starts_with('.')
        || Path::new(scope).extension().is_some()
}

fn path_scope_matches(scope: &str, target: &Path) -> bool {
    let target_candidates = path_match_candidates(target);
    path_scope_candidates(scope).iter().any(|scope_candidate| {
        target_candidates.iter().any(|target_candidate| {
            target_candidate.starts_with(scope_candidate)
                || scope_candidate.starts_with(target_candidate)
        })
    })
}

fn path_scope_candidates(scope: &str) -> Vec<PathBuf> {
    let mut candidates = vec![PathBuf::from(scope)];
    if let Ok(canonical) = Path::new(scope).canonicalize() {
        if !candidates.iter().any(|candidate| candidate == &canonical) {
            candidates.push(canonical);
        }
    }
    candidates
}

fn path_match_candidates(path: &Path) -> Vec<PathBuf> {
    let mut candidates = vec![path.to_path_buf()];
    if let Ok(canonical) = path.canonicalize() {
        if !candidates.iter().any(|candidate| candidate == &canonical) {
            candidates.push(canonical);
        }
    }
    candidates
}

fn collect_package_doc_paths(docs_dir: &Path) -> Vec<PathBuf> {
    let mut docs = Vec::new();
    collect_package_doc_paths_recursive(docs_dir, &mut docs);
    docs.sort();
    docs
}

fn collect_package_doc_paths_recursive(dir: &Path, docs: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };

    let mut entries = entries
        .flatten()
        .map(|entry| entry.path())
        .collect::<Vec<_>>();
    entries.sort();

    for path in entries {
        if path.is_dir() {
            collect_package_doc_paths_recursive(&path, docs);
            continue;
        }

        if path.extension().and_then(|ext| ext.to_str()) == Some("md") {
            docs.push(path);
        }
    }
}

fn render_package_docs_index(crate_name: &str, doc_path: &str, docs_dir: &Path) -> String {
    let section_links = collect_section_links(docs_dir);
    let mut rendered = String::new();

    rendered.push_str(&format!("# {crate_name}: Map of Content\n\n"));
    rendered.push_str(":PROPERTIES:\n");
    rendered.push_str(&format!(":ID: {}\n", derive_opaque_doc_id(doc_path)));
    rendered.push_str(":TYPE: INDEX\n");
    rendered.push_str(":STATUS: ACTIVE\n");
    rendered.push_str(":END:\n\n");
    rendered.push_str(&format!(
        "Standardized documentation index for the `{crate_name}` package.\n\n"
    ));

    if section_links.is_empty() {
        rendered.push_str(
            "Populate package-local documentation sections under this directory and extend this index as the package surface evolves.\n",
        );
        rendered.push_str("\n---\n\n");
        rendered.push_str(&render_index_footer());
        return rendered;
    }

    for (section, links) in &section_links {
        rendered.push_str(&format!("## {section}\n\n"));
        for link in links {
            rendered.push_str(&format!("- [[{link}]]\n"));
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

fn render_section_landing_page(
    crate_name: &str,
    crate_dir: &Path,
    doc_path: &str,
    spec: &SectionSpec,
) -> String {
    let mut rendered = String::new();
    rendered.push_str(&format!("# {}\n\n", spec.title));
    rendered.push_str(":PROPERTIES:\n");
    rendered.push_str(&format!(":ID: {}\n", derive_opaque_doc_id(doc_path)));
    rendered.push_str(&format!(":TYPE: {}\n", spec.doc_type));
    rendered.push_str(":STATUS: DRAFT\n");
    rendered.push_str(":END:\n\n");
    rendered.push_str(&format!(
        "{}\n\n",
        render_section_summary(crate_name, crate_dir, spec)
    ));
    rendered.push_str(&format!("{}\n", render_section_prompt(crate_name, spec)));
    rendered
}

fn render_index_footer() -> String {
    render_index_footer_with_values("v2.0", "pending")
}

fn render_index_footer_with_values(standards: &str, last_sync: &str) -> String {
    format!(":FOOTER:\n:STANDARDS: {standards}\n:LAST_SYNC: {last_sync}\n:END:\n")
}

fn link_target(relative_path: &str) -> String {
    relative_path
        .strip_suffix(".md")
        .unwrap_or(relative_path)
        .replace('\\', "/")
}

fn collect_index_body_links(lines: &[LineSlice<'_>]) -> Vec<String> {
    let relations_start = lines
        .iter()
        .position(|line| line.trimmed == ":RELATIONS:")
        .unwrap_or(lines.len());

    let mut links = Vec::new();
    for line in &lines[..relations_start] {
        if !line.trimmed.starts_with("- ") {
            continue;
        }
        for link in extract_wikilinks(line.without_newline) {
            if !links.iter().any(|existing| existing == &link) {
                links.push(link);
            }
        }
    }
    links
}

fn extract_wikilinks(content: &str) -> Vec<String> {
    let mut links = Vec::new();
    let mut remaining = content;

    while let Some(start) = remaining.find("[[") {
        let after_start = &remaining[start + 2..];
        let Some(end) = after_start.find("]]") else {
            break;
        };
        let link = &after_start[..end];
        if !link.is_empty() {
            links.push(link.to_string());
        }
        remaining = &after_start[end + 2..];
    }

    links
}

fn parse_relations_links_line<'a>(lines: &'a [LineSlice<'a>]) -> Option<LinksLine<'a>> {
    let relations_idx = lines
        .iter()
        .position(|line| line.trimmed == ":RELATIONS:")?;
    for line in lines.iter().skip(relations_idx + 1) {
        if line.trimmed == ":END:" {
            break;
        }

        if let Some(rest) = line.without_newline.strip_prefix(":LINKS:") {
            let leading_spaces = rest.chars().take_while(|ch| *ch == ' ').count();
            let value = &rest[leading_spaces..];
            let value_start = line.start_offset + ":LINKS:".len() + leading_spaces;
            let value_end = line.start_offset + line.without_newline.len();
            return Some(LinksLine {
                line: line.line_number,
                value,
                value_start,
                value_end,
            });
        }
    }
    None
}

fn parse_footer_block<'a>(lines: &'a [LineSlice<'a>]) -> Option<FooterBlock<'a>> {
    let footer_idx = lines.iter().position(|line| line.trimmed == ":FOOTER:")?;
    let footer_line = &lines[footer_idx];
    let mut standards_value = None;
    let mut last_sync_value = None;

    for line in lines.iter().skip(footer_idx + 1) {
        if line.trimmed == ":END:" {
            return Some(FooterBlock {
                line: footer_line.line_number,
                start_offset: footer_line.start_offset,
                end_offset: line.end_offset,
                standards_value,
                last_sync_value,
            });
        }

        if let Some(rest) = line.without_newline.strip_prefix(":STANDARDS:") {
            standards_value = Some(rest.trim());
            continue;
        }

        if let Some(rest) = line.without_newline.strip_prefix(":LAST_SYNC:") {
            last_sync_value = Some(rest.trim());
        }
    }

    None
}

fn plan_index_relations_block_insertion(
    index_content: &str,
    body_links: &[String],
) -> (IssueLocation, String) {
    let lines = collect_lines(index_content);
    let insertion_line = lines
        .iter()
        .find(|line| line.trimmed == "---" || line.trimmed == ":FOOTER:");
    let insert_offset = insertion_line.map_or(index_content.len(), |line| line.start_offset);
    let prefix = if insert_offset == 0 {
        ""
    } else if index_content[..insert_offset].ends_with("\n\n") {
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

fn plan_index_footer_block_insertion(index_content: &str) -> (IssueLocation, String) {
    let lines = collect_lines(index_content);
    let insert_offset = index_content.len();
    let prefix = if index_content.is_empty() {
        ""
    } else if index_content.ends_with("\n\n") {
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

fn plan_index_section_link_insertion(
    index_content: &str,
    spec: &SectionSpec,
    link_target: &str,
) -> (IssueLocation, String) {
    let lines = collect_lines(index_content);

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
    let prefix = if index_content.is_empty() {
        ""
    } else if insert_offset > 0 && index_content[..insert_offset].ends_with("\n\n") {
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

#[derive(Debug, Clone)]
struct SectionSpec {
    section_name: &'static str,
    relative_path: String,
    title: &'static str,
    doc_type: &'static str,
}

fn standard_section_specs(crate_name: &str) -> Vec<SectionSpec> {
    let slug = crate_slug(crate_name);
    vec![
        SectionSpec {
            section_name: "01_core",
            relative_path: format!("01_core/101_{slug}_core_boundary.md"),
            title: "Core Boundary",
            doc_type: "CORE",
        },
        SectionSpec {
            section_name: "03_features",
            relative_path: format!("03_features/201_{slug}_feature_ledger.md"),
            title: "Feature Ledger",
            doc_type: "FEATURE",
        },
        SectionSpec {
            section_name: "05_research",
            relative_path: format!("05_research/301_{slug}_research_agenda.md"),
            title: "Research Agenda",
            doc_type: "RESEARCH",
        },
        SectionSpec {
            section_name: "06_roadmap",
            relative_path: format!("06_roadmap/401_{slug}_roadmap.md"),
            title: "Roadmap",
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

pub(crate) fn is_package_local_crate_doc(doc_path: &str) -> bool {
    let path = Path::new(doc_path);
    if path.extension().and_then(|ext| ext.to_str()) != Some("md") {
        return false;
    }

    let components: Vec<String> = path
        .components()
        .filter_map(|component| match component {
            Component::Normal(part) => Some(part.to_string_lossy().into_owned()),
            _ => None,
        })
        .collect();

    components
        .windows(5)
        .any(|window| matches!(window, [a, b, c, _, d] if a == "packages" && b == "rust" && c == "crates" && d == "docs"))
}

#[derive(Debug, Clone, Copy)]
struct TopPropertiesDrawer<'a> {
    properties_line: usize,
    insert_offset: usize,
    newline: &'a str,
    id_line: Option<IdLine<'a>>,
}

#[derive(Debug, Clone, Copy)]
struct IdLine<'a> {
    line: usize,
    value: &'a str,
    value_start: usize,
    value_end: usize,
}

#[derive(Debug, Clone, Copy)]
struct LinksLine<'a> {
    line: usize,
    value: &'a str,
    value_start: usize,
    value_end: usize,
}

#[derive(Debug, Clone, Copy)]
struct FooterBlock<'a> {
    line: usize,
    start_offset: usize,
    end_offset: usize,
    standards_value: Option<&'a str>,
    last_sync_value: Option<&'a str>,
}

fn parse_top_properties_drawer(content: &str) -> Option<TopPropertiesDrawer<'_>> {
    let lines = collect_lines(content);

    let title_index = lines.iter().position(|line| !line.trimmed.is_empty())?;
    let title = lines.get(title_index)?;
    if !title.trimmed.starts_with('#') {
        return None;
    }

    let mut cursor = title_index + 1;
    while cursor < lines.len() && lines[cursor].trimmed.is_empty() {
        cursor += 1;
    }

    let properties = lines.get(cursor)?;
    if properties.trimmed != ":PROPERTIES:" {
        return None;
    }

    let newline = properties.newline;
    let insert_offset = properties.end_offset;

    let mut id_line = None;
    for line in lines.iter().skip(cursor + 1) {
        if line.trimmed == ":END:" {
            return Some(TopPropertiesDrawer {
                properties_line: properties.line_number,
                insert_offset,
                newline,
                id_line,
            });
        }

        if let Some(rest) = line.without_newline.strip_prefix(":ID:") {
            let leading_spaces = rest.chars().take_while(|ch| *ch == ' ').count();
            let value_start = line.start_offset + 4 + leading_spaces;
            let value = rest[leading_spaces..].trim();
            let value_end = value_start + value.len();
            id_line = Some(IdLine {
                line: line.line_number,
                value,
                value_start,
                value_end,
            });
        }
    }

    None
}

#[derive(Debug, Clone, Copy)]
struct LineSlice<'a> {
    line_number: usize,
    start_offset: usize,
    end_offset: usize,
    trimmed: &'a str,
    without_newline: &'a str,
    newline: &'a str,
}

fn collect_lines(content: &str) -> Vec<LineSlice<'_>> {
    let mut lines = Vec::new();
    let mut offset = 0usize;

    for (line_number, raw_line) in content.split_inclusive('\n').enumerate() {
        let without_newline = raw_line.trim_end_matches(['\n', '\r']);
        let newline = &raw_line[without_newline.len()..];
        lines.push(LineSlice {
            line_number: line_number + 1,
            start_offset: offset,
            end_offset: offset + raw_line.len(),
            trimmed: without_newline.trim(),
            without_newline,
            newline,
        });
        offset += raw_line.len();
    }

    if !content.is_empty() && !content.ends_with('\n') {
        let without_newline = content.rsplit_once('\n').map_or(content, |(_, tail)| tail);
        if lines.is_empty()
            || lines
                .last()
                .is_some_and(|line| line.end_offset != content.len())
        {
            let start_offset = content.len() - without_newline.len();
            lines.push(LineSlice {
                line_number: lines.len() + 1,
                start_offset,
                end_offset: content.len(),
                trimmed: without_newline.trim(),
                without_newline,
                newline: "",
            });
        }
    }

    lines
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::link_graph::LinkGraphIndex;
    use crate::zhenfa_router::native::audit::fix::AtomicFixBatch;
    use crate::zhenfa_router::native::audit::generate_surgical_fixes;
    use crate::zhenfa_router::native::semantic_check::{
        CheckType, WendaoSemanticCheckArgs, run_audit_core,
    };
    use std::collections::HashMap;
    use std::fs;
    use tempfile::TempDir;
    use xiuxian_zhenfa::ZhenfaContext;

    #[test]
    fn detects_non_opaque_doc_identity_for_package_local_docs() {
        let content = "# Title\n\n:PROPERTIES:\n:ID: readable-id\n:TYPE: CORE\n:END:\n";
        let doc_path = "packages/rust/crates/demo/docs/01_core/101_test.md";
        let issues = collect_doc_governance_issues(doc_path, content);
        let expected = derive_opaque_doc_id(doc_path);

        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].issue_type, DOC_IDENTITY_PROTOCOL_ISSUE_TYPE);
        assert_eq!(issues[0].suggestion.as_deref(), Some(expected.as_str()));
        assert_eq!(issues[0].location.as_ref().map(|loc| loc.line), Some(4));
    }

    #[test]
    fn detects_missing_doc_identity_inside_top_properties_drawer() {
        let content = "# Title\n\n:PROPERTIES:\n:TYPE: CORE\n:END:\n";
        let doc_path = "packages/rust/crates/demo/docs/01_core/101_test.md";
        let issues = collect_doc_governance_issues(doc_path, content);
        let expected = format!(":ID: {}\n", derive_opaque_doc_id(doc_path));

        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].issue_type, DOC_IDENTITY_PROTOCOL_ISSUE_TYPE);
        assert_eq!(issues[0].suggestion.as_deref(), Some(expected.as_str()));
        assert_eq!(issues[0].location.as_ref().map(|loc| loc.line), Some(4));
        assert_eq!(
            issues[0].location.as_ref().and_then(|loc| loc.byte_range),
            Some((22, 22))
        );
    }

    #[test]
    fn ignores_docs_outside_package_local_crate_docs() {
        let content = "# Title\n\n:PROPERTIES:\n:ID: readable-id\n:TYPE: CORE\n:END:\n";
        let issues = collect_doc_governance_issues("docs/notes.md", content);
        assert!(issues.is_empty());
    }

    #[test]
    fn surgical_fixes_repair_non_opaque_doc_identity() {
        let doc_key =
            "packages/rust/crates/demo/docs/01_core/101_external_modelica_plugin_boundary.md";
        let original = "# Demo\n\n:PROPERTIES:\n:ID: readable-id\n:TYPE: CORE\n:END:\n\nBody.\n";
        let issues = collect_doc_governance_issues(doc_key, original);
        assert_eq!(issues.len(), 1);

        let file_contents = HashMap::from([(doc_key.to_string(), original.to_string())]);
        let fixes = generate_surgical_fixes(&issues, &file_contents);
        assert_eq!(fixes.len(), 1);

        let mut content = original.to_string();
        let result = fixes[0].apply_surgical(&mut content);
        assert!(matches!(
            result,
            crate::zhenfa_router::native::audit::FixResult::Success
        ));
        assert!(content.contains(&format!(":ID: {}", derive_opaque_doc_id(&doc_key))));
    }

    #[test]
    fn detects_missing_package_docs_index_for_workspace_crate_docs() {
        let temp = TempDir::new().expect("tempdir");
        let crate_dir = temp.path().join("packages/rust/crates/demo");
        fs::create_dir_all(&crate_dir).expect("create crate dir");
        fs::write(
            crate_dir.join("Cargo.toml"),
            "[package]\nname = \"demo\"\nversion = \"0.1.0\"\n",
        )
        .expect("write cargo");
        let docs_dir = temp.path().join("packages/rust/crates/demo/docs/01_core");
        fs::create_dir_all(&docs_dir).expect("create docs dir");
        let doc_path = docs_dir.join("101_intro.md");
        let doc_path_str = doc_path.to_string_lossy().to_string();
        let content = format!(
            "# Intro\n\n:PROPERTIES:\n:ID: {}\n:END:\n\nIntro.\n",
            derive_opaque_doc_id(&doc_path_str)
        );
        fs::write(&doc_path, content).expect("write doc");

        let issues = collect_workspace_doc_governance_issues(temp.path(), None);
        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].issue_type, MISSING_PACKAGE_DOCS_INDEX_ISSUE_TYPE);
        assert!(
            issues[0]
                .doc
                .ends_with("packages/rust/crates/demo/docs/index.md")
        );
        let suggestion = issues[0].suggestion.as_ref().expect("suggestion");
        assert!(suggestion.contains("# demo: Map of Content"));
        assert!(suggestion.contains("[[01_core/101_intro]]"));
    }

    #[test]
    fn detects_missing_package_docs_tree_for_workspace_crate() {
        let temp = TempDir::new().expect("tempdir");
        let crate_dir = temp.path().join("packages/rust/crates/demo");
        fs::create_dir_all(&crate_dir).expect("create crate dir");
        fs::write(
            crate_dir.join("Cargo.toml"),
            "[package]\nname = \"demo\"\nversion = \"0.1.0\"\n",
        )
        .expect("write cargo");

        let issues = collect_workspace_doc_governance_issues(temp.path(), None);
        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].issue_type, MISSING_PACKAGE_DOCS_TREE_ISSUE_TYPE);
        assert_eq!(issues[0].severity, "warning");
        assert!(
            issues[0]
                .doc
                .ends_with("packages/rust/crates/demo/docs/index.md")
        );
        let suggestion = issues[0].suggestion.as_ref().expect("suggestion");
        assert!(suggestion.contains("# demo: Map of Content"));
        assert!(suggestion.contains("Standardized documentation index"));
    }

    #[test]
    fn detects_doc_identity_for_workspace_package_docs_tree_files() {
        let temp = TempDir::new().expect("tempdir");
        let crate_dir = temp.path().join("packages/rust/crates/demo");
        let core_dir = crate_dir.join("docs/01_core");
        fs::create_dir_all(&core_dir).expect("create core docs dir");
        fs::write(
            crate_dir.join("Cargo.toml"),
            "[package]\nname = \"demo\"\nversion = \"0.1.0\"\n",
        )
        .expect("write cargo");

        let index_path = crate_dir.join("docs/index.md");
        let index_path_str = index_path.to_string_lossy().to_string();
        fs::write(
            &index_path,
            format!(
                "# Demo\n\n:PROPERTIES:\n:ID: {}\n:TYPE: INDEX\n:END:\n",
                derive_opaque_doc_id(&index_path_str)
            ),
        )
        .expect("write index");

        let intro_path = core_dir.join("101_intro.md");
        let intro_path_str = intro_path.to_string_lossy().to_string();
        fs::write(
            &intro_path,
            "# Intro\n\n:PROPERTIES:\n:ID: readable-intro\n:TYPE: CORE\n:END:\n",
        )
        .expect("write intro");

        let issues = collect_workspace_doc_governance_issues(temp.path(), None);
        let issue = issues
            .iter()
            .find(|issue| {
                issue.issue_type == DOC_IDENTITY_PROTOCOL_ISSUE_TYPE && issue.doc == intro_path_str
            })
            .expect("workspace doc identity issue");

        assert_eq!(issue.severity, "error");
        assert_eq!(
            issue.suggestion.as_deref(),
            Some(derive_opaque_doc_id(&intro_path_str).as_str())
        );
    }

    #[test]
    fn workspace_doc_identity_scan_respects_explicit_doc_scope() {
        let temp = TempDir::new().expect("tempdir");
        let crate_dir = temp.path().join("packages/rust/crates/demo");
        let core_dir = crate_dir.join("docs/01_core");
        fs::create_dir_all(&core_dir).expect("create core docs dir");
        fs::write(
            crate_dir.join("Cargo.toml"),
            "[package]\nname = \"demo\"\nversion = \"0.1.0\"\n",
        )
        .expect("write cargo");

        let index_path = crate_dir.join("docs/index.md");
        let index_path_str = index_path.to_string_lossy().to_string();
        fs::write(
            &index_path,
            format!(
                "# Demo\n\n:PROPERTIES:\n:ID: {}\n:TYPE: INDEX\n:END:\n",
                derive_opaque_doc_id(&index_path_str)
            ),
        )
        .expect("write index");

        let intro_path = core_dir.join("101_intro.md");
        let intro_path_str = intro_path.to_string_lossy().to_string();
        fs::write(
            &intro_path,
            "# Intro\n\n:PROPERTIES:\n:ID: readable-intro\n:TYPE: CORE\n:END:\n",
        )
        .expect("write intro");

        let contracts_path = core_dir.join("102_contracts.md");
        fs::write(
            &contracts_path,
            "# Contracts\n\n:PROPERTIES:\n:ID: readable-contracts\n:TYPE: CORE\n:END:\n",
        )
        .expect("write contracts");

        let issues = collect_workspace_doc_governance_issues(temp.path(), Some(&intro_path_str));
        let identity_issues = issues
            .iter()
            .filter(|issue| issue.issue_type == DOC_IDENTITY_PROTOCOL_ISSUE_TYPE)
            .collect::<Vec<_>>();

        assert_eq!(identity_issues.len(), 1);
        assert_eq!(
            Path::new(&identity_issues[0].doc)
                .canonicalize()
                .expect("canonical issue path"),
            intro_path.canonicalize().expect("canonical intro path")
        );
    }

    #[test]
    fn workspace_scope_does_not_match_prefix_sibling_crates() {
        let temp = TempDir::new().expect("tempdir");
        let wendao_dir = temp.path().join("packages/rust/crates/xiuxian-wendao");
        let modelica_dir = temp
            .path()
            .join("packages/rust/crates/xiuxian-wendao-modelica");

        for crate_dir in [&wendao_dir, &modelica_dir] {
            fs::create_dir_all(crate_dir.join("docs/01_core")).expect("create docs dir");
            fs::write(
                crate_dir.join("Cargo.toml"),
                format!(
                    "[package]\nname = \"{}\"\nversion = \"0.1.0\"\n",
                    crate_dir
                        .file_name()
                        .and_then(|name| name.to_str())
                        .expect("crate name")
                ),
            )
            .expect("write cargo");
        }

        let wendao_doc = wendao_dir.join("docs/01_core/101_core.md");
        fs::write(
            &wendao_doc,
            "# Wendao\n\n:PROPERTIES:\n:ID: readable-wendao\n:TYPE: CORE\n:END:\n",
        )
        .expect("write wendao doc");

        let modelica_doc = modelica_dir.join("docs/01_core/101_core.md");
        fs::write(
            &modelica_doc,
            "# Modelica\n\n:PROPERTIES:\n:ID: readable-modelica\n:TYPE: CORE\n:END:\n",
        )
        .expect("write modelica doc");

        let issues = collect_workspace_doc_governance_issues(
            temp.path(),
            Some(&modelica_dir.join("docs").to_string_lossy()),
        );
        let identity_issues = issues
            .iter()
            .filter(|issue| issue.issue_type == DOC_IDENTITY_PROTOCOL_ISSUE_TYPE)
            .collect::<Vec<_>>();

        assert_eq!(identity_issues.len(), 1);
        assert_eq!(
            Path::new(&identity_issues[0].doc)
                .canonicalize()
                .expect("canonical issue path"),
            modelica_doc.canonicalize().expect("canonical modelica doc")
        );
    }

    #[test]
    fn run_audit_core_reports_missing_package_docs_index() {
        let temp = TempDir::new().expect("tempdir");
        let crate_dir = temp.path().join("packages/rust/crates/demo");
        fs::create_dir_all(&crate_dir).expect("create crate dir");
        fs::write(
            crate_dir.join("Cargo.toml"),
            "[package]\nname = \"demo\"\nversion = \"0.1.0\"\n",
        )
        .expect("write cargo");
        let docs_dir = temp.path().join("packages/rust/crates/demo/docs/01_core");
        fs::create_dir_all(&docs_dir).expect("create docs dir");
        let doc_path = docs_dir.join("101_intro.md");
        let doc_path_str = doc_path.to_string_lossy().to_string();
        let content = format!(
            "# Intro\n\n:PROPERTIES:\n:ID: {}\n:END:\n\nIntro.\n",
            derive_opaque_doc_id(&doc_path_str)
        );
        fs::write(&doc_path, content).expect("write doc");

        let index = LinkGraphIndex::build(temp.path()).expect("build index");
        let mut ctx = ZhenfaContext::default();
        ctx.insert_extension(index);

        let args = WendaoSemanticCheckArgs {
            doc: Some(".".to_string()),
            checks: Some(vec![CheckType::DocGovernance]),
            include_warnings: Some(true),
            source_paths: None,
            fuzzy_confidence_threshold: None,
        };
        let (issues, _file_contents) = run_audit_core(&ctx, &args).expect("audit");

        assert!(
            issues
                .iter()
                .any(|issue| issue.issue_type == MISSING_PACKAGE_DOCS_INDEX_ISSUE_TYPE)
        );
    }

    #[test]
    fn run_audit_core_reports_missing_package_docs_tree() {
        let temp = TempDir::new().expect("tempdir");
        let crate_dir = temp.path().join("packages/rust/crates/demo");
        fs::create_dir_all(&crate_dir).expect("create crate dir");
        fs::write(
            crate_dir.join("Cargo.toml"),
            "[package]\nname = \"demo\"\nversion = \"0.1.0\"\n",
        )
        .expect("write cargo");

        let index = LinkGraphIndex::build(temp.path()).expect("build index");
        let mut ctx = ZhenfaContext::default();
        ctx.insert_extension(index);

        let args = WendaoSemanticCheckArgs {
            doc: Some(".".to_string()),
            checks: Some(vec![CheckType::DocGovernance]),
            include_warnings: Some(true),
            source_paths: None,
            fuzzy_confidence_threshold: None,
        };
        let (issues, _file_contents) = run_audit_core(&ctx, &args).expect("audit");

        assert!(
            issues
                .iter()
                .any(|issue| issue.issue_type == MISSING_PACKAGE_DOCS_TREE_ISSUE_TYPE)
        );
    }

    #[test]
    fn detects_missing_standard_section_landings_for_existing_docs_tree() {
        let temp = TempDir::new().expect("tempdir");
        let crate_dir = temp.path().join("packages/rust/crates/demo");
        fs::create_dir_all(crate_dir.join("docs")).expect("create docs dir");
        fs::write(
            crate_dir.join("Cargo.toml"),
            "[package]\nname = \"demo\"\nversion = \"0.1.0\"\n",
        )
        .expect("write cargo");
        let index_path = crate_dir.join("docs/index.md");
        let index_path_str = index_path.to_string_lossy().to_string();
        fs::write(
            &index_path,
            format!(
                "# Demo\n\n:PROPERTIES:\n:ID: {}\n:END:\n",
                derive_opaque_doc_id(&index_path_str)
            ),
        )
        .expect("write index");

        let issues = collect_workspace_doc_governance_issues(temp.path(), None);
        let section_issues: Vec<_> = issues
            .iter()
            .filter(|issue| issue.issue_type == MISSING_PACKAGE_DOCS_SECTION_LANDING_ISSUE_TYPE)
            .collect();

        assert_eq!(section_issues.len(), 4);
        assert!(
            section_issues
                .iter()
                .any(|issue| issue.doc.ends_with("01_core/101_demo_core_boundary.md"))
        );
        assert!(section_issues.iter().any(|issue| {
            issue
                .doc
                .ends_with("03_features/201_demo_feature_ledger.md")
        }));
        assert!(section_issues.iter().any(|issue| {
            issue
                .doc
                .ends_with("05_research/301_demo_research_agenda.md")
        }));
        assert!(
            section_issues
                .iter()
                .any(|issue| issue.doc.ends_with("06_roadmap/401_demo_roadmap.md"))
        );
    }

    #[test]
    fn run_audit_core_reports_missing_standard_section_landings() {
        let temp = TempDir::new().expect("tempdir");
        let crate_dir = temp.path().join("packages/rust/crates/demo");
        fs::create_dir_all(crate_dir.join("docs")).expect("create docs dir");
        fs::write(
            crate_dir.join("Cargo.toml"),
            "[package]\nname = \"demo\"\nversion = \"0.1.0\"\n",
        )
        .expect("write cargo");
        let index_path = crate_dir.join("docs/index.md");
        let index_path_str = index_path.to_string_lossy().to_string();
        fs::write(
            &index_path,
            format!(
                "# Demo\n\n:PROPERTIES:\n:ID: {}\n:END:\n",
                derive_opaque_doc_id(&index_path_str)
            ),
        )
        .expect("write index");

        let index = LinkGraphIndex::build(temp.path()).expect("build index");
        let mut ctx = ZhenfaContext::default();
        ctx.insert_extension(index);

        let args = WendaoSemanticCheckArgs {
            doc: Some(".".to_string()),
            checks: Some(vec![CheckType::DocGovernance]),
            include_warnings: Some(true),
            source_paths: None,
            fuzzy_confidence_threshold: None,
        };
        let (issues, _file_contents) = run_audit_core(&ctx, &args).expect("audit");

        assert!(issues.iter().any(|issue| {
            issue.issue_type == MISSING_PACKAGE_DOCS_SECTION_LANDING_ISSUE_TYPE
                && issue
                    .doc
                    .ends_with("03_features/201_demo_feature_ledger.md")
        }));
    }

    #[test]
    fn detects_missing_standard_index_section_links_for_existing_landing_pages() {
        let temp = TempDir::new().expect("tempdir");
        let crate_dir = temp.path().join("packages/rust/crates/demo");
        let core_dir = crate_dir.join("docs/01_core");
        fs::create_dir_all(&core_dir).expect("create core docs dir");
        fs::write(
            crate_dir.join("Cargo.toml"),
            "[package]\nname = \"demo\"\nversion = \"0.1.0\"\n",
        )
        .expect("write cargo");

        let index_path = crate_dir.join("docs/index.md");
        let index_path_str = index_path.to_string_lossy().to_string();
        fs::write(
            &index_path,
            format!(
                "# Demo\n\n:PROPERTIES:\n:ID: {}\n:END:\n\n## 01_core: Architecture and Foundation\n\n",
                derive_opaque_doc_id(&index_path_str)
            ),
        )
        .expect("write index");

        let landing_path = core_dir.join("101_demo_core_boundary.md");
        let landing_path_str = landing_path.to_string_lossy().to_string();
        fs::write(
            &landing_path,
            format!(
                "# Core Boundary\n\n:PROPERTIES:\n:ID: {}\n:END:\n",
                derive_opaque_doc_id(&landing_path_str)
            ),
        )
        .expect("write landing");

        let issues = collect_workspace_doc_governance_issues(temp.path(), None);
        let link_issue = issues
            .iter()
            .find(|issue| issue.issue_type == MISSING_PACKAGE_DOCS_INDEX_SECTION_LINK_ISSUE_TYPE)
            .expect("missing index section-link issue");
        let expected_insert_offset = format!(
            "# Demo\n\n:PROPERTIES:\n:ID: {}\n:END:\n\n## 01_core: Architecture and Foundation\n\n",
            derive_opaque_doc_id(&index_path_str)
        )
        .len();

        assert_eq!(link_issue.doc, index_path_str);
        assert_eq!(link_issue.severity, "warning");
        assert_eq!(
            link_issue.suggestion.as_deref(),
            Some("- [[01_core/101_demo_core_boundary]]\n")
        );
        assert_eq!(
            link_issue
                .location
                .as_ref()
                .and_then(|location| location.byte_range),
            Some((expected_insert_offset, expected_insert_offset))
        );
    }

    #[test]
    fn detects_missing_standard_index_section_links_before_relations_or_footer_when_heading_missing()
     {
        let temp = TempDir::new().expect("tempdir");
        let crate_dir = temp.path().join("packages/rust/crates/demo");
        let feature_dir = crate_dir.join("docs/03_features");
        fs::create_dir_all(&feature_dir).expect("create feature docs dir");
        fs::write(
            crate_dir.join("Cargo.toml"),
            "[package]\nname = \"demo\"\nversion = \"0.1.0\"\n",
        )
        .expect("write cargo");

        let index_path = crate_dir.join("docs/index.md");
        let index_path_str = index_path.to_string_lossy().to_string();
        let index_content = format!(
            "# Demo\n\n:PROPERTIES:\n:ID: {}\n:END:\n\n## 01_core: Architecture and Foundation\n\n- [[01_core/101_demo_core_boundary]]\n\n:RELATIONS:\n:LINKS: [[01_core/101_demo_core_boundary]]\n:END:\n\n---\n\n:FOOTER:\n:STANDARDS: v2.0\n:LAST_SYNC: 2026-03-20\n:END:\n",
            derive_opaque_doc_id(&index_path_str)
        );
        fs::write(&index_path, &index_content).expect("write index");

        let landing_path = feature_dir.join("201_demo_feature_ledger.md");
        let landing_path_str = landing_path.to_string_lossy().to_string();
        fs::write(
            &landing_path,
            format!(
                "# Feature Ledger\n\n:PROPERTIES:\n:ID: {}\n:END:\n",
                derive_opaque_doc_id(&landing_path_str)
            ),
        )
        .expect("write landing");

        let issues = collect_workspace_doc_governance_issues(temp.path(), None);
        let link_issue = issues
            .iter()
            .find(|issue| {
                issue.issue_type == MISSING_PACKAGE_DOCS_INDEX_SECTION_LINK_ISSUE_TYPE
                    && issue.message.contains("03_features")
            })
            .expect("missing feature section-link issue");

        let relations_offset = index_content
            .find(":RELATIONS:")
            .expect("find relations block");

        assert_eq!(link_issue.doc, index_path_str);
        assert_eq!(
            link_issue.suggestion.as_deref(),
            Some("## 03_features\n\n- [[03_features/201_demo_feature_ledger]]\n\n")
        );
        assert_eq!(
            link_issue
                .location
                .as_ref()
                .and_then(|location| location.byte_range),
            Some((relations_offset, relations_offset))
        );
    }

    #[test]
    fn run_audit_core_reports_missing_standard_index_section_links() {
        let temp = TempDir::new().expect("tempdir");
        let crate_dir = temp.path().join("packages/rust/crates/demo");
        let core_dir = crate_dir.join("docs/01_core");
        fs::create_dir_all(&core_dir).expect("create core docs dir");
        fs::write(
            crate_dir.join("Cargo.toml"),
            "[package]\nname = \"demo\"\nversion = \"0.1.0\"\n",
        )
        .expect("write cargo");

        let index_path = crate_dir.join("docs/index.md");
        let index_path_str = index_path.to_string_lossy().to_string();
        fs::write(
            &index_path,
            format!(
                "# Demo\n\n:PROPERTIES:\n:ID: {}\n:END:\n\n",
                derive_opaque_doc_id(&index_path_str)
            ),
        )
        .expect("write index");

        let landing_path = core_dir.join("101_demo_core_boundary.md");
        let landing_path_str = landing_path.to_string_lossy().to_string();
        fs::write(
            &landing_path,
            format!(
                "# Core Boundary\n\n:PROPERTIES:\n:ID: {}\n:END:\n",
                derive_opaque_doc_id(&landing_path_str)
            ),
        )
        .expect("write landing");

        let index = LinkGraphIndex::build(temp.path()).expect("build index");
        let mut ctx = ZhenfaContext::default();
        ctx.insert_extension(index);

        let args = WendaoSemanticCheckArgs {
            doc: Some(".".to_string()),
            checks: Some(vec![CheckType::DocGovernance]),
            include_warnings: Some(true),
            source_paths: None,
            fuzzy_confidence_threshold: None,
        };
        let (issues, _file_contents) = run_audit_core(&ctx, &args).expect("audit");

        assert!(issues.iter().any(|issue| {
            issue.issue_type == MISSING_PACKAGE_DOCS_INDEX_SECTION_LINK_ISSUE_TYPE
                && issue
                    .doc
                    .ends_with("packages/rust/crates/demo/docs/index.md")
        }));
    }

    #[test]
    fn detects_missing_index_relation_links_for_existing_body_links() {
        let temp = TempDir::new().expect("tempdir");
        let crate_dir = temp.path().join("packages/rust/crates/demo");
        fs::create_dir_all(crate_dir.join("docs")).expect("create docs dir");
        fs::write(
            crate_dir.join("Cargo.toml"),
            "[package]\nname = \"demo\"\nversion = \"0.1.0\"\n",
        )
        .expect("write cargo");

        let index_path = crate_dir.join("docs/index.md");
        let index_path_str = index_path.to_string_lossy().to_string();
        let index_content = format!(
            "# Demo\n\n:PROPERTIES:\n:ID: {}\n:END:\n\n## 01_core\n\n- [[01_core/101_demo_core_boundary]]\n- [[01_core/102_demo_contracts]]\n\n:RELATIONS:\n:LINKS: [[01_core/101_demo_core_boundary]]\n:END:\n",
            derive_opaque_doc_id(&index_path_str)
        );
        fs::write(&index_path, &index_content).expect("write index");

        let issues = collect_workspace_doc_governance_issues(temp.path(), None);
        let relation_issue = issues
            .iter()
            .find(|issue| issue.issue_type == MISSING_PACKAGE_DOCS_INDEX_RELATION_LINK_ISSUE_TYPE)
            .expect("missing relation-link issue");

        assert_eq!(relation_issue.doc, index_path_str);
        assert_eq!(relation_issue.severity, "warning");
        assert!(
            relation_issue
                .message
                .contains("[[01_core/102_demo_contracts]]")
        );
        assert_eq!(
            relation_issue.suggestion.as_deref(),
            Some("[[01_core/101_demo_core_boundary]], [[01_core/102_demo_contracts]]")
        );
        let links_value = "[[01_core/101_demo_core_boundary]]";
        let links_line_start = index_content.find(":LINKS: ").expect("find links line");
        let value_start = links_line_start
            + ":LINKS: ".len()
            + index_content[links_line_start + ":LINKS: ".len()..]
                .find(links_value)
                .expect("find relation links value");
        let value_end = value_start + links_value.len();
        assert_eq!(
            relation_issue
                .location
                .as_ref()
                .and_then(|location| location.byte_range),
            Some((value_start, value_end))
        );
    }

    #[test]
    fn detects_missing_index_relations_block_for_existing_body_links() {
        let temp = TempDir::new().expect("tempdir");
        let crate_dir = temp.path().join("packages/rust/crates/demo");
        fs::create_dir_all(crate_dir.join("docs")).expect("create docs dir");
        fs::write(
            crate_dir.join("Cargo.toml"),
            "[package]\nname = \"demo\"\nversion = \"0.1.0\"\n",
        )
        .expect("write cargo");

        let index_path = crate_dir.join("docs/index.md");
        let index_path_str = index_path.to_string_lossy().to_string();
        let index_content = format!(
            "# Demo\n\n:PROPERTIES:\n:ID: {}\n:END:\n\n## 01_core\n\n- [[01_core/101_demo_core_boundary]]\n\n---\n\n:FOOTER:\n:STANDARDS: v2.0\n:END:\n",
            derive_opaque_doc_id(&index_path_str)
        );
        fs::write(&index_path, &index_content).expect("write index");

        let issues = collect_workspace_doc_governance_issues(temp.path(), None);
        let block_issue = issues
            .iter()
            .find(|issue| issue.issue_type == MISSING_PACKAGE_DOCS_INDEX_RELATIONS_BLOCK_ISSUE_TYPE)
            .expect("missing relations-block issue");

        assert_eq!(block_issue.doc, index_path_str);
        assert_eq!(block_issue.severity, "warning");
        assert!(
            block_issue
                .message
                .contains("[[01_core/101_demo_core_boundary]]")
        );
        assert_eq!(
            block_issue.suggestion.as_deref(),
            Some(":RELATIONS:\n:LINKS: [[01_core/101_demo_core_boundary]]\n:END:\n\n")
        );
        let insert_offset = index_content.find("---").expect("find footer separator");
        assert_eq!(
            block_issue
                .location
                .as_ref()
                .and_then(|location| location.byte_range),
            Some((insert_offset, insert_offset))
        );
    }

    #[test]
    fn detects_missing_index_footer_block_for_existing_relations_block() {
        let temp = TempDir::new().expect("tempdir");
        let crate_dir = temp.path().join("packages/rust/crates/demo");
        fs::create_dir_all(crate_dir.join("docs")).expect("create docs dir");
        fs::write(
            crate_dir.join("Cargo.toml"),
            "[package]\nname = \"demo\"\nversion = \"0.1.0\"\n",
        )
        .expect("write cargo");

        let index_path = crate_dir.join("docs/index.md");
        let index_path_str = index_path.to_string_lossy().to_string();
        let index_content = format!(
            "# Demo\n\n:PROPERTIES:\n:ID: {}\n:END:\n\n## 01_core\n\n- [[01_core/101_demo_core_boundary]]\n\n:RELATIONS:\n:LINKS: [[01_core/101_demo_core_boundary]]\n:END:\n",
            derive_opaque_doc_id(&index_path_str)
        );
        fs::write(&index_path, &index_content).expect("write index");

        let issues = collect_workspace_doc_governance_issues(temp.path(), None);
        let footer_issue = issues
            .iter()
            .find(|issue| issue.issue_type == MISSING_PACKAGE_DOCS_INDEX_FOOTER_BLOCK_ISSUE_TYPE)
            .expect("missing footer-block issue");

        assert_eq!(footer_issue.doc, index_path_str);
        assert_eq!(footer_issue.severity, "warning");
        assert!(footer_issue.message.contains(":FOOTER:"));
        assert_eq!(
            footer_issue.suggestion.as_deref(),
            Some("\n---\n\n:FOOTER:\n:STANDARDS: v2.0\n:LAST_SYNC: pending\n:END:\n")
        );
        assert_eq!(
            footer_issue
                .location
                .as_ref()
                .and_then(|location| location.byte_range),
            Some((index_content.len(), index_content.len()))
        );
    }

    #[test]
    fn detects_incomplete_index_footer_block_for_existing_footer() {
        let temp = TempDir::new().expect("tempdir");
        let crate_dir = temp.path().join("packages/rust/crates/demo");
        fs::create_dir_all(crate_dir.join("docs")).expect("create docs dir");
        fs::write(
            crate_dir.join("Cargo.toml"),
            "[package]\nname = \"demo\"\nversion = \"0.1.0\"\n",
        )
        .expect("write cargo");

        let index_path = crate_dir.join("docs/index.md");
        let index_path_str = index_path.to_string_lossy().to_string();
        let footer_block = ":FOOTER:\n:STANDARDS: v2.0\n:END:\n";
        let index_content = format!(
            "# Demo\n\n:PROPERTIES:\n:ID: {}\n:END:\n\n## 01_core\n\n- [[01_core/101_demo_core_boundary]]\n\n:RELATIONS:\n:LINKS: [[01_core/101_demo_core_boundary]]\n:END:\n\n---\n\n{footer_block}",
            derive_opaque_doc_id(&index_path_str)
        );
        fs::write(&index_path, &index_content).expect("write index");

        let issues = collect_workspace_doc_governance_issues(temp.path(), None);
        let footer_issue = issues
            .iter()
            .find(|issue| issue.issue_type == INCOMPLETE_PACKAGE_DOCS_INDEX_FOOTER_BLOCK_ISSUE_TYPE)
            .expect("missing incomplete footer-block issue");

        let footer_start = index_content.find(":FOOTER:").expect("find footer start");
        let footer_end = footer_start + footer_block.len();

        assert_eq!(footer_issue.doc, index_path_str);
        assert_eq!(footer_issue.severity, "warning");
        assert!(footer_issue.message.contains(":LAST_SYNC:"));
        assert_eq!(
            footer_issue.suggestion.as_deref(),
            Some(":FOOTER:\n:STANDARDS: v2.0\n:LAST_SYNC: pending\n:END:\n")
        );
        assert_eq!(
            footer_issue
                .location
                .as_ref()
                .and_then(|location| location.byte_range),
            Some((footer_start, footer_end))
        );
    }

    #[test]
    fn detects_stale_index_footer_standards_for_existing_footer() {
        let temp = TempDir::new().expect("tempdir");
        let crate_dir = temp.path().join("packages/rust/crates/demo");
        fs::create_dir_all(crate_dir.join("docs")).expect("create docs dir");
        fs::write(
            crate_dir.join("Cargo.toml"),
            "[package]\nname = \"demo\"\nversion = \"0.1.0\"\n",
        )
        .expect("write cargo");

        let index_path = crate_dir.join("docs/index.md");
        let index_path_str = index_path.to_string_lossy().to_string();
        let footer_block = ":FOOTER:\n:STANDARDS: v1.0\n:LAST_SYNC: 2026-03-20\n:END:\n";
        let index_content = format!(
            "# Demo\n\n:PROPERTIES:\n:ID: {}\n:END:\n\n## 01_core\n\n- [[01_core/101_demo_core_boundary]]\n\n:RELATIONS:\n:LINKS: [[01_core/101_demo_core_boundary]]\n:END:\n\n---\n\n{footer_block}",
            derive_opaque_doc_id(&index_path_str)
        );
        fs::write(&index_path, &index_content).expect("write index");

        let issues = collect_workspace_doc_governance_issues(temp.path(), None);
        let footer_issue = issues
            .iter()
            .find(|issue| issue.issue_type == STALE_PACKAGE_DOCS_INDEX_FOOTER_STANDARDS_ISSUE_TYPE)
            .expect("missing stale footer-standards issue");

        let footer_start = index_content.find(":FOOTER:").expect("find footer start");
        let footer_end = footer_start + footer_block.len();

        assert_eq!(footer_issue.doc, index_path_str);
        assert_eq!(footer_issue.severity, "warning");
        assert!(footer_issue.message.contains("v1.0"));
        assert_eq!(
            footer_issue.suggestion.as_deref(),
            Some(":FOOTER:\n:STANDARDS: v2.0\n:LAST_SYNC: 2026-03-20\n:END:\n")
        );
        assert_eq!(
            footer_issue
                .location
                .as_ref()
                .and_then(|location| location.byte_range),
            Some((footer_start, footer_end))
        );
    }

    #[test]
    fn run_audit_core_reports_missing_index_relation_links() {
        let temp = TempDir::new().expect("tempdir");
        let crate_dir = temp.path().join("packages/rust/crates/demo");
        fs::create_dir_all(crate_dir.join("docs")).expect("create docs dir");
        fs::write(
            crate_dir.join("Cargo.toml"),
            "[package]\nname = \"demo\"\nversion = \"0.1.0\"\n",
        )
        .expect("write cargo");

        let index_path = crate_dir.join("docs/index.md");
        let index_path_str = index_path.to_string_lossy().to_string();
        fs::write(
            &index_path,
            format!(
                "# Demo\n\n:PROPERTIES:\n:ID: {}\n:END:\n\n## 01_core\n\n- [[01_core/101_demo_core_boundary]]\n\n:RELATIONS:\n:LINKS: \n:END:\n",
                derive_opaque_doc_id(&index_path_str)
            ),
        )
        .expect("write index");

        let index = LinkGraphIndex::build(temp.path()).expect("build index");
        let mut ctx = ZhenfaContext::default();
        ctx.insert_extension(index);

        let args = WendaoSemanticCheckArgs {
            doc: Some(".".to_string()),
            checks: Some(vec![CheckType::DocGovernance]),
            include_warnings: Some(true),
            source_paths: None,
            fuzzy_confidence_threshold: None,
        };
        let (issues, _file_contents) = run_audit_core(&ctx, &args).expect("audit");

        assert!(issues.iter().any(|issue| {
            issue.issue_type == MISSING_PACKAGE_DOCS_INDEX_RELATION_LINK_ISSUE_TYPE
                && issue
                    .doc
                    .ends_with("packages/rust/crates/demo/docs/index.md")
        }));
    }

    #[test]
    fn detects_stale_index_relation_links_without_missing_links() {
        let temp = TempDir::new().expect("tempdir");
        let crate_dir = temp.path().join("packages/rust/crates/demo");
        fs::create_dir_all(crate_dir.join("docs")).expect("create docs dir");
        fs::write(
            crate_dir.join("Cargo.toml"),
            "[package]\nname = \"demo\"\nversion = \"0.1.0\"\n",
        )
        .expect("write cargo");

        let index_path = crate_dir.join("docs/index.md");
        let index_path_str = index_path.to_string_lossy().to_string();
        let index_content = format!(
            "# Demo\n\n:PROPERTIES:\n:ID: {}\n:END:\n\n## 01_core\n\n- [[01_core/101_demo_core_boundary]]\n\n:RELATIONS:\n:LINKS: [[01_core/101_demo_core_boundary]], [[01_core/999_stale]]\n:END:\n",
            derive_opaque_doc_id(&index_path_str)
        );
        fs::write(&index_path, &index_content).expect("write index");

        let issues = collect_workspace_doc_governance_issues(temp.path(), None);
        let stale_issue = issues
            .iter()
            .find(|issue| issue.issue_type == STALE_PACKAGE_DOCS_INDEX_RELATION_LINK_ISSUE_TYPE)
            .expect("missing stale relation-link issue");

        assert_eq!(stale_issue.doc, index_path_str);
        assert_eq!(stale_issue.severity, "warning");
        assert!(stale_issue.message.contains("[[01_core/999_stale]]"));
        assert_eq!(
            stale_issue.suggestion.as_deref(),
            Some("[[01_core/101_demo_core_boundary]]")
        );
        let relation_value = "[[01_core/101_demo_core_boundary]], [[01_core/999_stale]]";
        let links_line_start = index_content.find(":LINKS: ").expect("find links line");
        let value_start = links_line_start
            + ":LINKS: ".len()
            + index_content[links_line_start + ":LINKS: ".len()..]
                .find(relation_value)
                .expect("find stale relation links value");
        let value_end = value_start + relation_value.len();
        assert_eq!(
            stale_issue
                .location
                .as_ref()
                .and_then(|location| location.byte_range),
            Some((value_start, value_end))
        );
    }

    #[test]
    fn run_audit_core_reports_stale_index_relation_links() {
        let temp = TempDir::new().expect("tempdir");
        let crate_dir = temp.path().join("packages/rust/crates/demo");
        fs::create_dir_all(crate_dir.join("docs")).expect("create docs dir");
        fs::write(
            crate_dir.join("Cargo.toml"),
            "[package]\nname = \"demo\"\nversion = \"0.1.0\"\n",
        )
        .expect("write cargo");

        let index_path = crate_dir.join("docs/index.md");
        let index_path_str = index_path.to_string_lossy().to_string();
        fs::write(
            &index_path,
            format!(
                "# Demo\n\n:PROPERTIES:\n:ID: {}\n:END:\n\n## 01_core\n\n- [[01_core/101_demo_core_boundary]]\n\n:RELATIONS:\n:LINKS: [[01_core/101_demo_core_boundary]], [[01_core/999_stale]]\n:END:\n",
                derive_opaque_doc_id(&index_path_str)
            ),
        )
        .expect("write index");

        let index = LinkGraphIndex::build(temp.path()).expect("build index");
        let mut ctx = ZhenfaContext::default();
        ctx.insert_extension(index);

        let args = WendaoSemanticCheckArgs {
            doc: Some(".".to_string()),
            checks: Some(vec![CheckType::DocGovernance]),
            include_warnings: Some(true),
            source_paths: None,
            fuzzy_confidence_threshold: None,
        };
        let (issues, _file_contents) = run_audit_core(&ctx, &args).expect("audit");

        assert!(issues.iter().any(|issue| {
            issue.issue_type == STALE_PACKAGE_DOCS_INDEX_RELATION_LINK_ISSUE_TYPE
                && issue
                    .doc
                    .ends_with("packages/rust/crates/demo/docs/index.md")
        }));
    }

    #[test]
    fn run_audit_core_reports_missing_index_footer_block() {
        let temp = TempDir::new().expect("tempdir");
        let crate_dir = temp.path().join("packages/rust/crates/demo");
        fs::create_dir_all(crate_dir.join("docs")).expect("create docs dir");
        fs::write(
            crate_dir.join("Cargo.toml"),
            "[package]\nname = \"demo\"\nversion = \"0.1.0\"\n",
        )
        .expect("write cargo");

        let index_path = crate_dir.join("docs/index.md");
        let index_path_str = index_path.to_string_lossy().to_string();
        fs::write(
            &index_path,
            format!(
                "# Demo\n\n:PROPERTIES:\n:ID: {}\n:END:\n\n## 01_core\n\n- [[01_core/101_demo_core_boundary]]\n\n:RELATIONS:\n:LINKS: [[01_core/101_demo_core_boundary]]\n:END:\n",
                derive_opaque_doc_id(&index_path_str)
            ),
        )
        .expect("write index");

        let index = LinkGraphIndex::build(temp.path()).expect("build index");
        let mut ctx = ZhenfaContext::default();
        ctx.insert_extension(index);

        let args = WendaoSemanticCheckArgs {
            doc: Some(".".to_string()),
            checks: Some(vec![CheckType::DocGovernance]),
            include_warnings: Some(true),
            source_paths: None,
            fuzzy_confidence_threshold: None,
        };
        let (issues, _file_contents) = run_audit_core(&ctx, &args).expect("audit");

        assert!(issues.iter().any(|issue| {
            issue.issue_type == MISSING_PACKAGE_DOCS_INDEX_FOOTER_BLOCK_ISSUE_TYPE
                && issue
                    .doc
                    .ends_with("packages/rust/crates/demo/docs/index.md")
        }));
    }

    #[test]
    fn run_audit_core_reports_incomplete_index_footer_block() {
        let temp = TempDir::new().expect("tempdir");
        let crate_dir = temp.path().join("packages/rust/crates/demo");
        fs::create_dir_all(crate_dir.join("docs")).expect("create docs dir");
        fs::write(
            crate_dir.join("Cargo.toml"),
            "[package]\nname = \"demo\"\nversion = \"0.1.0\"\n",
        )
        .expect("write cargo");

        let index_path = crate_dir.join("docs/index.md");
        let index_path_str = index_path.to_string_lossy().to_string();
        fs::write(
            &index_path,
            format!(
                "# Demo\n\n:PROPERTIES:\n:ID: {}\n:END:\n\n## 01_core\n\n- [[01_core/101_demo_core_boundary]]\n\n:RELATIONS:\n:LINKS: [[01_core/101_demo_core_boundary]]\n:END:\n\n---\n\n:FOOTER:\n:STANDARDS: v2.0\n:END:\n",
                derive_opaque_doc_id(&index_path_str)
            ),
        )
        .expect("write index");

        let index = LinkGraphIndex::build(temp.path()).expect("build index");
        let mut ctx = ZhenfaContext::default();
        ctx.insert_extension(index);

        let args = WendaoSemanticCheckArgs {
            doc: Some(".".to_string()),
            checks: Some(vec![CheckType::DocGovernance]),
            include_warnings: Some(true),
            source_paths: None,
            fuzzy_confidence_threshold: None,
        };
        let (issues, _file_contents) = run_audit_core(&ctx, &args).expect("audit");

        assert!(issues.iter().any(|issue| {
            issue.issue_type == INCOMPLETE_PACKAGE_DOCS_INDEX_FOOTER_BLOCK_ISSUE_TYPE
                && issue
                    .doc
                    .ends_with("packages/rust/crates/demo/docs/index.md")
        }));
    }

    #[test]
    fn run_audit_core_reports_stale_index_footer_standards() {
        let temp = TempDir::new().expect("tempdir");
        let crate_dir = temp.path().join("packages/rust/crates/demo");
        fs::create_dir_all(crate_dir.join("docs")).expect("create docs dir");
        fs::write(
            crate_dir.join("Cargo.toml"),
            "[package]\nname = \"demo\"\nversion = \"0.1.0\"\n",
        )
        .expect("write cargo");

        let index_path = crate_dir.join("docs/index.md");
        let index_path_str = index_path.to_string_lossy().to_string();
        fs::write(
            &index_path,
            format!(
                "# Demo\n\n:PROPERTIES:\n:ID: {}\n:END:\n\n## 01_core\n\n- [[01_core/101_demo_core_boundary]]\n\n:RELATIONS:\n:LINKS: [[01_core/101_demo_core_boundary]]\n:END:\n\n---\n\n:FOOTER:\n:STANDARDS: v1.0\n:LAST_SYNC: 2026-03-20\n:END:\n",
                derive_opaque_doc_id(&index_path_str)
            ),
        )
        .expect("write index");

        let index = LinkGraphIndex::build(temp.path()).expect("build index");
        let mut ctx = ZhenfaContext::default();
        ctx.insert_extension(index);

        let args = WendaoSemanticCheckArgs {
            doc: Some(".".to_string()),
            checks: Some(vec![CheckType::DocGovernance]),
            include_warnings: Some(true),
            source_paths: None,
            fuzzy_confidence_threshold: None,
        };
        let (issues, _file_contents) = run_audit_core(&ctx, &args).expect("audit");

        assert!(issues.iter().any(|issue| {
            issue.issue_type == STALE_PACKAGE_DOCS_INDEX_FOOTER_STANDARDS_ISSUE_TYPE
                && issue
                    .doc
                    .ends_with("packages/rust/crates/demo/docs/index.md")
        }));
    }

    #[test]
    fn run_audit_core_loads_explicit_workspace_doc_file_for_fix_generation() {
        let temp = TempDir::new().expect("tempdir");
        let crate_dir = temp.path().join("packages/rust/crates/demo");
        fs::create_dir_all(crate_dir.join("docs")).expect("create docs dir");
        fs::write(
            crate_dir.join("Cargo.toml"),
            "[package]\nname = \"demo\"\nversion = \"0.1.0\"\n",
        )
        .expect("write cargo");

        let index_path = crate_dir.join("docs/index.md");
        let index_path_str = index_path.to_string_lossy().to_string();
        fs::write(
            &index_path,
            format!(
                "# Demo\n\n:PROPERTIES:\n:ID: {}\n:END:\n\n## 01_core\n\n- [[01_core/101_demo_core_boundary]]\n\n:RELATIONS:\n:LINKS: [[01_core/101_demo_core_boundary]]\n:END:\n\n---\n\n:FOOTER:\n:STANDARDS: v1.0\n:LAST_SYNC: 2026-03-20\n:END:\n",
                derive_opaque_doc_id(&index_path_str)
            ),
        )
        .expect("write index");

        let index = LinkGraphIndex::build(temp.path()).expect("build index");
        let mut ctx = ZhenfaContext::default();
        ctx.insert_extension(index);

        let args = WendaoSemanticCheckArgs {
            doc: Some(index_path_str.clone()),
            checks: Some(vec![CheckType::DocGovernance]),
            include_warnings: Some(true),
            source_paths: None,
            fuzzy_confidence_threshold: None,
        };
        let (_issues, file_contents) = run_audit_core(&ctx, &args).expect("audit");

        assert!(file_contents.contains_key(&index_path_str));

        let issues = collect_stale_index_footer_standards(&index_path);
        assert_eq!(issues.len(), 1);

        let fixes = generate_surgical_fixes(&issues, &file_contents);
        assert_eq!(fixes.len(), 1);
        assert_eq!(
            fixes[0].issue_type,
            STALE_PACKAGE_DOCS_INDEX_FOOTER_STANDARDS_ISSUE_TYPE
        );
    }

    #[test]
    fn run_audit_core_reports_doc_identity_for_explicit_workspace_doc_file() {
        let temp = TempDir::new().expect("tempdir");
        let crate_dir = temp.path().join("packages/rust/crates/demo");
        fs::create_dir_all(crate_dir.join("docs")).expect("create docs dir");
        fs::write(
            crate_dir.join("Cargo.toml"),
            "[package]\nname = \"demo\"\nversion = \"0.1.0\"\n",
        )
        .expect("write cargo");

        let index_path = crate_dir.join("docs/index.md");
        let index_path_str = index_path.to_string_lossy().to_string();
        fs::write(
            &index_path,
            "# Demo\n\n:PROPERTIES:\n:ID: readable-demo-index\n:TYPE: INDEX\n:END:\n",
        )
        .expect("write index");

        let index = LinkGraphIndex::build(temp.path()).expect("build index");
        let mut ctx = ZhenfaContext::default();
        ctx.insert_extension(index);

        let args = WendaoSemanticCheckArgs {
            doc: Some(index_path_str.clone()),
            checks: Some(vec![CheckType::DocGovernance]),
            include_warnings: Some(true),
            source_paths: None,
            fuzzy_confidence_threshold: None,
        };
        let (issues, _file_contents) = run_audit_core(&ctx, &args).expect("audit");

        let issue = issues
            .iter()
            .find(|issue| issue.issue_type == DOC_IDENTITY_PROTOCOL_ISSUE_TYPE)
            .expect("doc identity issue");
        let canonical_index_path = index_path
            .canonicalize()
            .expect("canonical index path")
            .to_string_lossy()
            .to_string();
        let expected_id = derive_opaque_doc_id(&canonical_index_path);
        assert_eq!(issue.doc, canonical_index_path);
        assert_eq!(issue.severity, "error");
        assert_eq!(issue.suggestion.as_deref(), Some(expected_id.as_str()));
    }

    #[test]
    fn run_audit_core_seeds_workspace_doc_identity_issue_files_for_fix_generation() {
        let temp = TempDir::new().expect("tempdir");
        let crate_dir = temp.path().join("packages/rust/crates/demo");
        let core_dir = crate_dir.join("docs/01_core");
        fs::create_dir_all(&core_dir).expect("create core docs dir");
        fs::write(
            crate_dir.join("Cargo.toml"),
            "[package]\nname = \"demo\"\nversion = \"0.1.0\"\n",
        )
        .expect("write cargo");

        let index_path = crate_dir.join("docs/index.md");
        let index_path_str = index_path.to_string_lossy().to_string();
        fs::write(
            &index_path,
            format!(
                "# Demo\n\n:PROPERTIES:\n:ID: {}\n:TYPE: INDEX\n:END:\n",
                derive_opaque_doc_id(&index_path_str)
            ),
        )
        .expect("write index");

        let intro_path = core_dir.join("101_intro.md");
        fs::write(
            &intro_path,
            "# Intro\n\n:PROPERTIES:\n:ID: readable-intro\n:TYPE: CORE\n:END:\n",
        )
        .expect("write intro");

        let index = LinkGraphIndex::build(temp.path()).expect("build index");
        let mut ctx = ZhenfaContext::default();
        ctx.insert_extension(index);

        let docs_scope = crate_dir.join("docs").to_string_lossy().to_string();
        let args = WendaoSemanticCheckArgs {
            doc: Some(docs_scope),
            checks: Some(vec![CheckType::DocGovernance]),
            include_warnings: Some(true),
            source_paths: None,
            fuzzy_confidence_threshold: None,
        };
        let (issues, file_contents) = run_audit_core(&ctx, &args).expect("audit");
        let canonical_intro_path = intro_path
            .canonicalize()
            .expect("canonical intro path")
            .to_string_lossy()
            .to_string();

        let identity_issue = issues
            .iter()
            .find(|issue| {
                issue.issue_type == DOC_IDENTITY_PROTOCOL_ISSUE_TYPE
                    && issue.doc == canonical_intro_path
            })
            .expect("workspace doc identity issue");

        assert!(file_contents.contains_key(&canonical_intro_path));

        let fixes = generate_surgical_fixes(&[identity_issue.clone()], &file_contents);
        assert_eq!(fixes.len(), 1);
        assert_eq!(fixes[0].issue_type, DOC_IDENTITY_PROTOCOL_ISSUE_TYPE);
    }

    #[test]
    fn package_docs_directory_scope_fix_rewrites_doc_identity_issues_end_to_end() {
        let temp = TempDir::new().expect("tempdir");
        let crate_dir = temp.path().join("packages/rust/crates/demo");
        let core_dir = crate_dir.join("docs/01_core");
        let feature_dir = crate_dir.join("docs/03_features");
        fs::create_dir_all(&core_dir).expect("create core docs dir");
        fs::create_dir_all(&feature_dir).expect("create feature docs dir");
        fs::write(
            crate_dir.join("Cargo.toml"),
            "[package]\nname = \"demo\"\nversion = \"0.1.0\"\n",
        )
        .expect("write cargo");

        let index_path = crate_dir.join("docs/index.md");
        let index_path_str = index_path.to_string_lossy().to_string();
        fs::write(
            &index_path,
            format!(
                "# Demo\n\n:PROPERTIES:\n:ID: {}\n:TYPE: INDEX\n:END:\n",
                derive_opaque_doc_id(&index_path_str)
            ),
        )
        .expect("write index");

        let core_doc = core_dir.join("101_intro.md");
        let core_doc_str = core_doc.to_string_lossy().to_string();
        fs::write(
            &core_doc,
            "# Intro\n\n:PROPERTIES:\n:ID: readable-intro\n:TYPE: CORE\n:END:\n",
        )
        .expect("write core doc");

        let feature_doc = feature_dir.join("201_feature_ledger.md");
        let feature_doc_str = feature_doc.to_string_lossy().to_string();
        fs::write(
            &feature_doc,
            "# Feature Ledger\n\n:PROPERTIES:\n:ID: readable-feature-ledger\n:TYPE: FEATURE\n:END:\n",
        )
        .expect("write feature doc");

        let index = LinkGraphIndex::build(temp.path()).expect("build index");
        let mut ctx = ZhenfaContext::default();
        ctx.insert_extension(index);

        let args = WendaoSemanticCheckArgs {
            doc: Some(crate_dir.join("docs").to_string_lossy().to_string()),
            checks: Some(vec![CheckType::DocGovernance]),
            include_warnings: Some(true),
            source_paths: None,
            fuzzy_confidence_threshold: None,
        };
        let (issues, file_contents) = run_audit_core(&ctx, &args).expect("audit");

        let doc_identity_issues = issues
            .iter()
            .filter(|issue| issue.issue_type == DOC_IDENTITY_PROTOCOL_ISSUE_TYPE)
            .cloned()
            .collect::<Vec<_>>();
        assert_eq!(doc_identity_issues.len(), 2);

        let fixes = generate_surgical_fixes(&doc_identity_issues, &file_contents);
        assert_eq!(fixes.len(), 2);

        let report = AtomicFixBatch::new(fixes).apply_all();
        assert!(report.is_success(), "{}", report.summary());

        let core_doc_content = fs::read_to_string(&core_doc).expect("read core doc");
        assert!(
            core_doc_content.contains(&format!(":ID: {}", derive_opaque_doc_id(&core_doc_str)))
        );

        let feature_doc_content = fs::read_to_string(&feature_doc).expect("read feature doc");
        assert!(
            feature_doc_content
                .contains(&format!(":ID: {}", derive_opaque_doc_id(&feature_doc_str)))
        );
    }

    #[test]
    fn run_audit_core_reports_missing_index_relations_block() {
        let temp = TempDir::new().expect("tempdir");
        let crate_dir = temp.path().join("packages/rust/crates/demo");
        fs::create_dir_all(crate_dir.join("docs")).expect("create docs dir");
        fs::write(
            crate_dir.join("Cargo.toml"),
            "[package]\nname = \"demo\"\nversion = \"0.1.0\"\n",
        )
        .expect("write cargo");

        let index_path = crate_dir.join("docs/index.md");
        let index_path_str = index_path.to_string_lossy().to_string();
        fs::write(
            &index_path,
            format!(
                "# Demo\n\n:PROPERTIES:\n:ID: {}\n:END:\n\n## 01_core\n\n- [[01_core/101_demo_core_boundary]]\n",
                derive_opaque_doc_id(&index_path_str)
            ),
        )
        .expect("write index");

        let index = LinkGraphIndex::build(temp.path()).expect("build index");
        let mut ctx = ZhenfaContext::default();
        ctx.insert_extension(index);

        let args = WendaoSemanticCheckArgs {
            doc: Some(".".to_string()),
            checks: Some(vec![CheckType::DocGovernance]),
            include_warnings: Some(true),
            source_paths: None,
            fuzzy_confidence_threshold: None,
        };
        let (issues, _file_contents) = run_audit_core(&ctx, &args).expect("audit");

        assert!(issues.iter().any(|issue| {
            issue.issue_type == MISSING_PACKAGE_DOCS_INDEX_RELATIONS_BLOCK_ISSUE_TYPE
                && issue
                    .doc
                    .ends_with("packages/rust/crates/demo/docs/index.md")
        }));
    }
}
