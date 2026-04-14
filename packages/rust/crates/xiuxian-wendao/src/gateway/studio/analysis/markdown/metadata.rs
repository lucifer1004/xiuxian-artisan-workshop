use std::collections::HashSet;
use std::path::Path;

use serde_yaml::Value;
use xiuxian_wendao_parsers::{
    MarkdownSection, MarkdownWikiLink, extract_wikilinks, parse_markdown_note,
    parse_wikilink_literal,
};

use crate::gateway::studio::pathing::{
    normalize_path_like, studio_display_path, studio_project_name,
};
use crate::gateway::studio::router::StudioState;
use crate::gateway::studio::types::{
    MarkdownAnalysisDocumentLink, MarkdownAnalysisDocumentLinkKind,
    MarkdownAnalysisDocumentMetadata,
};
use crate::link_graph::{LinkGraphDirection, LinkGraphIndex};
use crate::parsers::docs_governance::{collect_lines, parse_relations_links_line};
use crate::parsers::markdown::parse_property_relations;

pub(crate) fn build_markdown_document_metadata(
    state: &StudioState,
    path: &str,
    content: &str,
    index: Option<&LinkGraphIndex>,
) -> MarkdownAnalysisDocumentMetadata {
    let fallback_title = Path::new(path)
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or("Markdown document");
    let note = parse_markdown_note(content, fallback_title);
    let raw_metadata = note.document.raw_metadata.as_ref();
    let document_core = &note.document.core;
    let doc_attributes = note
        .core
        .sections
        .first()
        .map(|section| section.attributes());
    let current_doc_id = index.and_then(|graph| resolve_current_doc_id(state, graph, path));
    let current_path = normalize_path_like(path).unwrap_or_else(|| path.trim().to_string());
    let current_title = document_core.title.clone();

    MarkdownAnalysisDocumentMetadata {
        doc_id: current_doc_id.clone(),
        title: document_core.title.clone(),
        tags: extract_document_tags(document_core.tags.as_slice(), doc_attributes),
        doc_type: extract_document_type(document_core.doc_type.as_deref(), doc_attributes),
        updated: extract_updated_string(raw_metadata, doc_attributes),
        parent: extract_parent_link(
            state,
            index,
            note.core.sections.as_slice(),
            current_doc_id.as_deref(),
            current_path.as_str(),
            current_title.as_str(),
        ),
        outgoing_links: collect_outgoing_links(
            state,
            index,
            content,
            current_doc_id.as_deref(),
            current_path.as_str(),
            current_title.as_str(),
        ),
        backlinks: collect_backlinks(state, index, current_doc_id.as_deref()),
    }
}

fn extract_document_tags(
    parser_tags: &[String],
    doc_attributes: Option<&std::collections::HashMap<String, String>>,
) -> Vec<String> {
    if !parser_tags.is_empty() {
        return parser_tags.to_vec();
    }

    let mut tags = doc_attributes
        .and_then(|attributes| attributes.get("TAGS"))
        .map(|raw| {
            raw.split([',', ';'])
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(ToOwned::to_owned)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    tags.sort();
    tags.dedup();
    tags
}

fn extract_document_type(
    parser_doc_type: Option<&str>,
    doc_attributes: Option<&std::collections::HashMap<String, String>>,
) -> Option<String> {
    parser_doc_type.map(ToOwned::to_owned).or_else(|| {
        doc_attributes
            .and_then(|attributes| attributes.get("TYPE").or_else(|| attributes.get("KIND")))
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty())
    })
}

fn extract_updated_string(
    raw_metadata: Option<&Value>,
    doc_attributes: Option<&std::collections::HashMap<String, String>>,
) -> Option<String> {
    const UPDATED_KEYS: &[&str] = &["updated", "updated_at", "modified", "modified_at"];
    const ATTRIBUTE_KEYS: &[&str] = &["UPDATED", "UPDATED_AT", "MODIFIED", "MODIFIED_AT"];

    if let Some(Value::Mapping(mapping)) = raw_metadata {
        for key in UPDATED_KEYS {
            let Some(value) = mapping.get(&Value::String((*key).to_string())) else {
                continue;
            };
            match value {
                Value::String(raw) => {
                    let trimmed = raw.trim();
                    if !trimmed.is_empty() {
                        return Some(trimmed.to_string());
                    }
                }
                Value::Number(number) => return Some(number.to_string()),
                _ => {}
            }
        }
    }

    ATTRIBUTE_KEYS.iter().find_map(|key| {
        doc_attributes
            .and_then(|attributes| attributes.get(*key))
            .map(|value| value.trim())
            .filter(|value| !value.is_empty())
            .map(ToOwned::to_owned)
    })
}

fn extract_parent_link(
    state: &StudioState,
    index: Option<&LinkGraphIndex>,
    sections: &[MarkdownSection],
    current_doc_id: Option<&str>,
    current_path: &str,
    current_title: &str,
) -> Option<MarkdownAnalysisDocumentLink> {
    let raw_parent = sections
        .iter()
        .find_map(|section| section.attributes().get("PARENT"))?;
    let parsed = parse_wikilink_literal(raw_parent.trim())?;

    Some(link_from_wikilink(
        state,
        index,
        MarkdownAnalysisDocumentLinkKind::Parent,
        None,
        None,
        current_doc_id,
        current_path,
        current_title,
        &parsed,
    ))
}

fn collect_outgoing_links(
    state: &StudioState,
    index: Option<&LinkGraphIndex>,
    content: &str,
    current_doc_id: Option<&str>,
    current_path: &str,
    current_title: &str,
) -> Vec<MarkdownAnalysisDocumentLink> {
    let mut rows = Vec::new();
    let mut seen = HashSet::new();

    for relation in parse_property_relations(content) {
        let mut row = link_from_relation_target(
            state,
            index,
            current_doc_id,
            current_path,
            current_title,
            relation.target.note_target.as_deref(),
            relation
                .target
                .address
                .as_ref()
                .map(crate::link_graph::addressing::Address::to_display_string),
            relation.target.original.clone(),
        );
        row.kind = MarkdownAnalysisDocumentLinkKind::Relation;
        row.relation_type = Some(relation.relation_type.to_string());
        row.metadata_owner = relation.source.scope_display();
        push_unique_link(&mut rows, &mut seen, row);
    }

    let lines = collect_lines(content);
    if let Some(links_line) = parse_relations_links_line(lines.as_slice()) {
        for wikilink in extract_wikilinks(links_line.value) {
            let row = link_from_wikilink(
                state,
                index,
                MarkdownAnalysisDocumentLinkKind::Index,
                None,
                Some("Index Relations".to_string()),
                current_doc_id,
                current_path,
                current_title,
                &wikilink,
            );
            push_unique_link(&mut rows, &mut seen, row);
        }
    }

    rows
}

fn collect_backlinks(
    state: &StudioState,
    index: Option<&LinkGraphIndex>,
    current_doc_id: Option<&str>,
) -> Vec<MarkdownAnalysisDocumentLink> {
    let Some(graph) = index else {
        return Vec::new();
    };
    let Some(doc_id) = current_doc_id else {
        return Vec::new();
    };

    graph
        .neighbors(doc_id, LinkGraphDirection::Incoming, 1, usize::MAX)
        .into_iter()
        .map(|neighbor| MarkdownAnalysisDocumentLink {
            label: neighbor.title.clone(),
            kind: MarkdownAnalysisDocumentLinkKind::Backlink,
            literal: None,
            relation_type: None,
            metadata_owner: None,
            doc_id: graph
                .resolve_doc_id_pub(neighbor.path.as_str())
                .map(str::to_string),
            path: Some(studio_display_path(state, neighbor.path.as_str())),
            title: Some(neighbor.title),
            target_address: None,
        })
        .collect()
}

fn push_unique_link(
    rows: &mut Vec<MarkdownAnalysisDocumentLink>,
    seen: &mut HashSet<String>,
    row: MarkdownAnalysisDocumentLink,
) {
    let key = format!(
        "{:?}|{}|{}|{}|{}|{}",
        row.kind,
        row.label,
        row.doc_id.as_deref().unwrap_or(""),
        row.path.as_deref().unwrap_or(""),
        row.target_address.as_deref().unwrap_or(""),
        row.relation_type.as_deref().unwrap_or(""),
    );
    if seen.insert(key) {
        rows.push(row);
    }
}

fn link_from_wikilink(
    state: &StudioState,
    index: Option<&LinkGraphIndex>,
    kind: MarkdownAnalysisDocumentLinkKind,
    relation_type: Option<String>,
    metadata_owner: Option<String>,
    current_doc_id: Option<&str>,
    current_path: &str,
    current_title: &str,
    wikilink: &MarkdownWikiLink,
) -> MarkdownAnalysisDocumentLink {
    let alias_label = extract_wikilink_alias(wikilink.original.as_str());
    let original = Some(wikilink.original.clone());
    let target = &wikilink.addressed_target;
    build_link_row(
        state,
        index,
        kind,
        relation_type,
        metadata_owner,
        current_doc_id,
        current_path,
        current_title,
        target.target.as_deref(),
        target.target_address.clone(),
        alias_label,
        original,
    )
}

fn link_from_relation_target(
    state: &StudioState,
    index: Option<&LinkGraphIndex>,
    current_doc_id: Option<&str>,
    current_path: &str,
    current_title: &str,
    note_target: Option<&str>,
    target_address: Option<String>,
    original: String,
) -> MarkdownAnalysisDocumentLink {
    build_link_row(
        state,
        index,
        MarkdownAnalysisDocumentLinkKind::Relation,
        None,
        None,
        current_doc_id,
        current_path,
        current_title,
        note_target,
        target_address,
        None,
        Some(original),
    )
}

#[allow(clippy::too_many_arguments)]
fn build_link_row(
    state: &StudioState,
    index: Option<&LinkGraphIndex>,
    kind: MarkdownAnalysisDocumentLinkKind,
    relation_type: Option<String>,
    metadata_owner: Option<String>,
    current_doc_id: Option<&str>,
    current_path: &str,
    current_title: &str,
    note_target: Option<&str>,
    target_address: Option<String>,
    alias_label: Option<String>,
    literal: Option<String>,
) -> MarkdownAnalysisDocumentLink {
    let resolved = resolve_link_target(
        state,
        index,
        current_doc_id,
        current_path,
        current_title,
        note_target,
        target_address.clone(),
    );

    let label = alias_label
        .or_else(|| resolved.title.clone())
        .or_else(|| resolved.path.clone())
        .or_else(|| note_target.map(ToOwned::to_owned))
        .or_else(|| target_address.clone())
        .unwrap_or_else(|| literal.clone().unwrap_or_else(|| "link".to_string()));

    MarkdownAnalysisDocumentLink {
        label,
        kind,
        literal,
        relation_type,
        metadata_owner,
        doc_id: resolved.doc_id,
        path: resolved.path,
        title: resolved.title,
        target_address: target_address.or(resolved.target_address),
    }
}

struct ResolvedLinkTarget {
    doc_id: Option<String>,
    path: Option<String>,
    title: Option<String>,
    target_address: Option<String>,
}

fn resolve_link_target(
    state: &StudioState,
    index: Option<&LinkGraphIndex>,
    current_doc_id: Option<&str>,
    current_path: &str,
    current_title: &str,
    note_target: Option<&str>,
    target_address: Option<String>,
) -> ResolvedLinkTarget {
    let Some(raw_target) = note_target.map(str::trim).filter(|value| !value.is_empty()) else {
        return ResolvedLinkTarget {
            doc_id: current_doc_id.map(ToOwned::to_owned),
            path: Some(current_path.to_string()),
            title: Some(current_title.to_string()),
            target_address,
        };
    };

    let Some(graph) = index else {
        return ResolvedLinkTarget {
            doc_id: None,
            path: Some(raw_target.to_string()),
            title: None,
            target_address,
        };
    };

    let Some(doc_id) = graph.resolve_doc_id_pub(raw_target).map(str::to_string) else {
        return ResolvedLinkTarget {
            doc_id: None,
            path: Some(raw_target.to_string()),
            title: None,
            target_address,
        };
    };

    let doc = match graph.get_doc(doc_id.as_str()) {
        Some(doc) => doc,
        None => {
            return ResolvedLinkTarget {
                doc_id: Some(doc_id),
                path: Some(raw_target.to_string()),
                title: None,
                target_address,
            };
        }
    };

    ResolvedLinkTarget {
        doc_id: Some(doc_id),
        path: Some(studio_display_path(state, doc.path.as_str())),
        title: Some(doc.title.clone()),
        target_address,
    }
}

fn resolve_current_doc_id(
    state: &StudioState,
    index: &LinkGraphIndex,
    path: &str,
) -> Option<String> {
    let normalized = normalize_path_like(path)?;
    let unscoped = if let Some(project_name) = studio_project_name(state, normalized.as_str()) {
        normalized
            .strip_prefix(format!("{project_name}/").as_str())
            .unwrap_or(normalized.as_str())
            .to_string()
    } else {
        normalized
    };

    index
        .resolve_doc_id_pub(unscoped.as_str())
        .or_else(|| index.resolve_doc_id_pub(path))
        .map(str::to_string)
}

fn extract_wikilink_alias(original: &str) -> Option<String> {
    let inner = original
        .trim()
        .strip_prefix("[[")?
        .strip_suffix("]]")?
        .trim();
    let (_, alias) = inner.split_once('|')?;
    let alias = alias.trim();
    (!alias.is_empty()).then(|| alias.to_string())
}
