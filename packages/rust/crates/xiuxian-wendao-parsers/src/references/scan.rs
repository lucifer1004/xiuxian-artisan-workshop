use comrak::{
    Arena, Options,
    nodes::{AstNode, NodeValue},
    parse_document,
};

use crate::AddressedTarget;
use crate::sourcepos::sourcepos_to_byte_range;

use super::types::{MarkdownReference, MarkdownReferenceKind};

pub(super) fn extract_references_with_comrak(markdown: &str) -> Vec<MarkdownReference> {
    let mut options = Options::default();
    options.extension.wikilinks_title_after_pipe = true;

    let arena = Arena::new();
    let root = parse_document(&arena, markdown, &options);

    root.descendants()
        .filter_map(|node| parse_reference_node(node, markdown))
        .collect()
}

fn parse_reference_node(node: &AstNode<'_>, markdown: &str) -> Option<MarkdownReference> {
    let data = node.data();
    let (kind, raw_target) = match &data.value {
        NodeValue::Link(link) => (MarkdownReferenceKind::Markdown, link.url.as_str()),
        NodeValue::WikiLink(link) if !is_embedded_wikilink(node) => {
            (MarkdownReferenceKind::WikiLink, link.url.as_str())
        }
        _ => return None,
    };

    let addressed_target = parse_reference_target(raw_target)?;
    let (start, end) = sourcepos_to_byte_range(markdown, data.sourcepos)?;
    let original = markdown.get(start..end)?.to_string();

    Some(MarkdownReference::new(kind, addressed_target, original))
}

fn parse_reference_target(raw: &str) -> Option<AddressedTarget> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return None;
    }

    if trimmed.starts_with('#') {
        return Some(AddressedTarget::new(None, Some(trimmed.to_string())));
    }

    let Some((target, raw_address)) = trimmed.split_once('#') else {
        return Some(AddressedTarget::new(Some(trimmed.to_string()), None));
    };

    let target = target.trim();
    if target.is_empty() {
        return None;
    }

    let raw_address = raw_address.trim();
    let target_address = if raw_address.is_empty() {
        None
    } else {
        Some(format!("#{raw_address}"))
    };

    Some(AddressedTarget::new(
        Some(target.to_string()),
        target_address,
    ))
}

fn is_embedded_wikilink(node: &AstNode<'_>) -> bool {
    let Some(previous) = node.previous_sibling() else {
        return false;
    };
    let NodeValue::Text(text) = &previous.data().value else {
        return false;
    };
    text.as_ref().ends_with('!')
}
