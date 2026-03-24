//! Semantic check routines.

use std::path::Path;

use crate::link_graph::parser::CodeObservation;
use crate::link_graph::{PageIndexNode, RegistryBuildResult, RegistryIndex};
use crate::zhenfa_router::native::audit::{SourceFile, suggest_pattern_fix_with_threshold};

use super::parsing::{
    extract_hash_references, extract_id_references, generate_suggested_id, validate_contract,
};
use super::types::{FuzzySuggestionData, IssueLocation, NodeStatus, SemanticIssue, attrs};

/// Check for dead links (references to non-existent IDs).
pub(super) fn check_dead_links(
    node: &PageIndexNode,
    doc_id: &str,
    registry: &RegistryIndex,
    issues: &mut Vec<SemanticIssue>,
) {
    let id_refs = extract_id_references(&node.text);

    for entity in id_refs {
        let target_id = &entity[1..];
        if !registry.contains(target_id) {
            issues.push(SemanticIssue {
                severity: "error".to_string(),
                issue_type: "dead_link".to_string(),
                doc: doc_id.to_string(),
                node_id: node.node_id.clone(),
                message: format!("Dead link: reference to non-existent ID '{target_id}'"),
                location: Some(IssueLocation::from_node(node)),
                suggestion: Some(format!(
                    "Remove the reference or create a node with :ID: {target_id}"
                )),
                fuzzy_suggestion: None,
            });
        }
    }
}

/// Check for references to deprecated nodes.
pub(super) fn check_deprecated_refs(
    node: &PageIndexNode,
    doc_id: &str,
    registry: &RegistryIndex,
    issues: &mut Vec<SemanticIssue>,
) {
    let id_refs = extract_id_references(&node.text);

    for entity in id_refs {
        let target_id = &entity[1..];
        if let Some(indexed) = registry.get(target_id)
            && let Some(status_str) = indexed.node.metadata.attributes.get(attrs::STATUS)
            && NodeStatus::parse_lossy(status_str) == NodeStatus::Deprecated
        {
            issues.push(SemanticIssue {
                severity: "warning".to_string(),
                issue_type: "deprecated_ref".to_string(),
                doc: doc_id.to_string(),
                node_id: node.node_id.clone(),
                message: format!("Reference to deprecated node '{target_id}' (status: DEPRECATED)"),
                location: Some(IssueLocation::from_node(node)),
                suggestion: Some(format!(
                    "Update reference from deprecated node '{target_id}' to its replacement"
                )),
                fuzzy_suggestion: None,
            });
        }
    }
}

/// Check contract constraints.
pub(super) fn check_contracts(node: &PageIndexNode, doc_id: &str, issues: &mut Vec<SemanticIssue>) {
    if let Some(contract) = node.metadata.attributes.get(attrs::CONTRACT) {
        let content = &node.text;

        if let Some(violation) = validate_contract(contract, content) {
            issues.push(SemanticIssue {
                severity: "error".to_string(),
                issue_type: "contract_violation".to_string(),
                doc: doc_id.to_string(),
                node_id: node.node_id.clone(),
                message: format!("Contract violation: {violation} (contract: '{contract}')"),
                location: Some(IssueLocation::from_node(node)),
                suggestion: Some(
                    "Update the content to satisfy the contract constraint".to_string(),
                ),
                fuzzy_suggestion: None,
            });
        }
    }
}

/// Check hash alignment (`expect_hash` vs actual `content_hash`).
pub(super) fn check_hash_alignment(
    node: &PageIndexNode,
    doc_id: &str,
    registry: &RegistryIndex,
    issues: &mut Vec<SemanticIssue>,
) {
    let hash_refs = extract_hash_references(&node.text);

    for hash_ref in hash_refs {
        if let Some(expect_hash) = &hash_ref.expect_hash
            && let Some(indexed) = registry.get(&hash_ref.target_id)
        {
            if let Some(actual_hash) = &indexed.node.metadata.content_hash {
                if expect_hash != actual_hash {
                    issues.push(SemanticIssue {
                        severity: "warning".to_string(),
                        issue_type: "content_drift".to_string(),
                        doc: doc_id.to_string(),
                        node_id: node.node_id.clone(),
                        message: format!(
                            "Content drift: reference to '{}' expects hash '{}' but current hash is '{}'",
                            hash_ref.target_id, expect_hash, actual_hash
                        ),
                        location: Some(IssueLocation::from_node(node)),
                        suggestion: Some(format!(
                            "Update the reference hash to '{actual_hash}' or verify the content change is intentional"
                        )),
                        fuzzy_suggestion: None,
                    });
                }
            } else {
                issues.push(SemanticIssue {
                    severity: "info".to_string(),
                    issue_type: "missing_content_hash".to_string(),
                    doc: doc_id.to_string(),
                    node_id: node.node_id.clone(),
                    message: format!(
                        "Target '{}' has no content_hash for verification",
                        hash_ref.target_id
                    ),
                    location: Some(IssueLocation::from_node(node)),
                    suggestion: None,
                    fuzzy_suggestion: None,
                });
            }
        }
    }
}

/// Check for ID collisions (same ID in multiple documents).
pub(super) fn check_id_collisions(
    build_result: &RegistryBuildResult,
    issues: &mut Vec<SemanticIssue>,
) {
    for collision in &build_result.collisions {
        let locations_str = collision
            .locations
            .iter()
            .map(|(doc_id, path)| format!("{}:{}", doc_id, path.join("/")))
            .collect::<Vec<_>>()
            .join(", ");

        let (primary_doc, primary_path) = &collision.locations[0];

        issues.push(SemanticIssue {
            severity: "error".to_string(),
            issue_type: "id_collision".to_string(),
            doc: primary_doc.clone(),
            node_id: collision.id.clone(),
            message: format!(
                "ID collision: '{}' appears in {} locations: {}",
                collision.id,
                collision.locations.len(),
                locations_str
            ),
            location: Some(IssueLocation {
                line: 0,
                heading_path: primary_path.join(" / "),
                byte_range: None,
            }),
            suggestion: Some(
                "Rename one of the nodes to have a unique ID, or remove duplicate :ID: attributes"
                    .to_string(),
            ),
            fuzzy_suggestion: None,
        });
    }
}

/// Check for missing mandatory :ID: property drawer (Blueprint v2.2).
pub(super) fn check_missing_identity(
    node: &PageIndexNode,
    doc_id: &str,
    issues: &mut Vec<SemanticIssue>,
) {
    let should_have_id = node.level <= 2;

    if should_have_id && !node.metadata.attributes.contains_key(attrs::ID) {
        issues.push(SemanticIssue {
            severity: "warning".to_string(),
            issue_type: "missing_identity".to_string(),
            doc: doc_id.to_string(),
            node_id: node.node_id.clone(),
            message: format!(
                "Heading '{}' at level {} lacks explicit :ID: property drawer",
                node.title, node.level
            ),
            location: Some(IssueLocation::from_node(node)),
            suggestion: Some(format!(
                "Add a property drawer with :ID: {} to enable stable anchoring",
                generate_suggested_id(&node.title)
            )),
            fuzzy_suggestion: None,
        });
    }
}

/// Check for legacy syntax markers (Blueprint v2.2).
pub(super) fn check_legacy_syntax(
    node: &PageIndexNode,
    doc_id: &str,
    issues: &mut Vec<SemanticIssue>,
) {
    let text = &node.text;

    let legacy_patterns = [
        ("SEE ALSO", "Use `[[#id]]` wiki-links instead"),
        ("RELATED TO", "Use `[[#id]]` wiki-links instead"),
        (
            "<<",
            "Use `[[#id]]` for internal links instead of <<legacy>> syntax",
        ),
    ];

    for (pattern, suggestion) in legacy_patterns {
        if text.contains(pattern) {
            issues.push(SemanticIssue {
                severity: "warning".to_string(),
                issue_type: "legacy_syntax".to_string(),
                doc: doc_id.to_string(),
                node_id: node.node_id.clone(),
                message: format!("Legacy syntax '{pattern}' detected"),
                location: Some(IssueLocation::from_node(node)),
                suggestion: Some(suggestion.to_string()),
                fuzzy_suggestion: None,
            });
        }
    }
}

fn push_invalid_observation_language_issue(
    node: &PageIndexNode,
    doc_id: &str,
    obs: &CodeObservation,
    issues: &mut Vec<SemanticIssue>,
) {
    issues.push(SemanticIssue {
        severity: "error".to_string(),
        issue_type: "invalid_observation_language".to_string(),
        doc: doc_id.to_string(),
        node_id: node.node_id.clone(),
        message: format!(
            "Unsupported language '{}' in :OBSERVE: pattern",
            obs.language
        ),
        location: Some(IssueLocation::from_node(node)),
        suggestion: Some(
            "Use a supported language: rust, python, javascript, typescript, go, java, c, cpp, etc.".to_string()
        ),
        fuzzy_suggestion: None,
    });
}

fn build_observation_fuzzy_suggestion(
    obs: &CodeObservation,
    lang: xiuxian_ast::Lang,
    source_files: &[SourceFile],
    fuzzy_threshold: Option<f32>,
) -> Option<FuzzySuggestionData> {
    if source_files.is_empty() {
        return None;
    }

    suggest_pattern_fix_with_threshold(&obs.pattern, lang, source_files, fuzzy_threshold)
        .map(|suggestion| FuzzySuggestionData::from_suggestion(suggestion, obs.pattern.clone()))
}

fn format_observation_source_location(source_location: Option<&str>) -> String {
    source_location.map_or_else(String::new, |location| {
        format!("Found similar code at: {location}")
    })
}

fn format_observation_suggestion(
    pattern: &str,
    description: &str,
    fuzzy_suggestion_data: Option<&FuzzySuggestionData>,
    fallback: &str,
) -> String {
    if let Some(data) = fuzzy_suggestion_data {
        format!(
            "Pattern '{pattern}' {description} {}\nConfidence: {:.0}%\n{}",
            data.suggested_pattern,
            data.confidence * 100.0,
            format_observation_source_location(data.source_location.as_deref())
        )
    } else {
        fallback.to_string()
    }
}

fn count_observation_matches(
    obs: &CodeObservation,
    lang: xiuxian_ast::Lang,
    source_files: &[SourceFile],
) -> usize {
    source_files
        .iter()
        .filter_map(|file| {
            let file_path = Path::new(&file.path);
            xiuxian_ast::Lang::from_path(file_path)
                .filter(|file_lang| *file_lang == lang)
                .and_then(|_| xiuxian_ast::scan(&file.content, &obs.pattern, lang).ok())
                .map(|matches| matches.len())
        })
        .sum()
}

/// Check :OBSERVE: code patterns for validity using xiuxian-ast (Blueprint v2.7).
pub(super) fn check_code_observations(
    node: &PageIndexNode,
    doc_id: &str,
    source_files: &[SourceFile],
    fuzzy_threshold: Option<f32>,
    issues: &mut Vec<SemanticIssue>,
) {
    for obs in &node.metadata.observations {
        let Some(lang) = obs.ast_language() else {
            push_invalid_observation_language_issue(node, doc_id, obs, issues);
            continue;
        };

        if let Err(error) = obs.validate_pattern() {
            let fuzzy_suggestion_data =
                build_observation_fuzzy_suggestion(obs, lang, source_files, fuzzy_threshold);
            let suggestion_text = format_observation_suggestion(
                &obs.pattern,
                "is invalid. Consider updating to:",
                fuzzy_suggestion_data.as_ref(),
                "Fix the pattern syntax or check xiuxian-ast documentation for valid sgrep patterns",
            );

            issues.push(SemanticIssue {
                severity: "error".to_string(),
                issue_type: "invalid_observation_pattern".to_string(),
                doc: doc_id.to_string(),
                node_id: node.node_id.clone(),
                message: format!("Invalid sgrep pattern in :OBSERVE:: {error}"),
                location: Some(IssueLocation::from_node(node)),
                suggestion: Some(suggestion_text),
                fuzzy_suggestion: fuzzy_suggestion_data,
            });
            continue;
        }

        if source_files.is_empty() || count_observation_matches(obs, lang, source_files) > 0 {
            continue;
        }

        let fuzzy_suggestion_data =
            build_observation_fuzzy_suggestion(obs, lang, source_files, fuzzy_threshold);
        let suggestion_text = format_observation_suggestion(
            &obs.pattern,
            "found no matches in the provided sources. Consider updating to:",
            fuzzy_suggestion_data.as_ref(),
            "Adjust the pattern or provide source files that contain the target code",
        );

        issues.push(SemanticIssue {
            severity: "warning".to_string(),
            issue_type: "observation_target_missing".to_string(),
            doc: doc_id.to_string(),
            node_id: node.node_id.clone(),
            message: format!(
                "Observation pattern '{}' found no matches in source files",
                obs.pattern
            ),
            location: Some(IssueLocation::from_node(node)),
            suggestion: Some(suggestion_text),
            fuzzy_suggestion: fuzzy_suggestion_data,
        });
    }
}
