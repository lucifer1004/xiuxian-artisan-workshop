//! Vision ingress operator for semantic injection.
//!
//! Scans all image attachments and calls external multimodal interfaces
//! (OCR/LLM Vision) to inject vision annotations into the graph index.
//!
//! ## Usage
//!
//! ```ignore
//! use crate::link_graph::index::build::vision_ingress::VisionIngress;
//!
//! let ingress = VisionIngress::new(Box::new(MyVisionProvider));
//! let annotations = ingress.process_attachments(&attachments, &docs).await;
//! ```

use super::saliency_snapshot::SaliencySnapshot;
use crate::link_graph::models::{
    LinkGraphAttachment, LinkGraphAttachmentKind, LinkGraphDocument, VisionAnnotation,
};
use std::collections::HashMap;
use std::path::PathBuf;

/// Vision provider trait for multimodal analysis.
pub trait VisionProvider: Send + Sync {
    /// Analyze an image and return vision annotation.
    async fn analyze(&self, path: &PathBuf) -> Option<VisionAnnotation>;
}

/// Default no-op vision provider.
#[derive(Debug, Clone)]
pub struct NoOpVisionProvider;

impl VisionProvider for NoOpVisionProvider {
    async fn analyze(&self, _path: &PathBuf) -> Option<VisionAnnotation> {
        None
    }
}

/// Vision ingress operator.
pub struct VisionIngress {
    provider: Box<dyn VisionProvider + Send + Sync>,
}

impl VisionIngress {
    /// Create new vision ingress with provider.
    pub fn new(provider: Box<dyn VisionProvider + Send + Sync>) -> Self {
        Self { provider }
    }

    /// Process all image attachments in the index.
    pub async fn process_attachments(
        &self,
        attachments: &[LinkGraphAttachment],
        docs_by_id: &HashMap<String, LinkGraphDocument>,
    ) -> HashMap<String, VisionAnnotation> {
        if attachments.is_empty() {
            return HashMap::new();
        }

        let mut results = HashMap::new();

        for attachment in attachments {
            if attachment.kind != LinkGraphAttachmentKind::Image {
                continue;
            }

            if let Some(full_path) = self.resolve_full_path(attachment) {
                if let Some(annotation) = self.provider.analyze(full_path).await {
                    results.insert(attachment.attachment_name.clone(), annotation);
                }
            }
        }

        results
    }

    fn resolve_full_path(&self, _attachment: &LinkGraphAttachment) -> Option<PathBuf> {
        // TODO: Wire up actual path resolution
        None
    }
}

/// Build semantic edges from vision annotations.
///
/// Creates edges between images and documents based on
/// vision-extracted text/entities matching document IDs.
#[must_use]
pub fn build_cross_modal_edges(
    annotations: &HashMap<String, VisionAnnotation>,
    doc_ids: &[String],
) -> HashMap<String, Vec<String>> {
    if annotations.is_empty() || doc_ids.is_empty() {
        return HashMap::new();
    }

    let mut edges: HashMap<String, Vec<String>> = HashMap::new();
    let doc_id_set: std::collections::HashSet<&str> = doc_ids.iter().map(|s| s.as_str()).collect();

    for (image_name, annotation) in annotations {
        // Match description words against doc IDs
        for word in annotation.description.split_whitespace() {
            let lower_word = word.to_lowercase();
            if doc_id_set.contains(lower_word.as_str()) {
                edges
                    .entry(image_name.clone())
                    .or_insert_with(Vec::new)
                    .push(lower_word);
            }
        }

        // Match entities against doc IDs
        for entity in &annotation.entities {
            let lower_entity = entity.to_lowercase();
            if doc_id_set.contains(lower_entity.as_str()) {
                edges
                    .entry(image_name.clone())
                    .or_insert_with(Vec::new)
                    .push(lower_entity);
            }
        }
    }

    edges
}

/// Process attachments with optional saliency snapshot.
pub fn process_attachments_with_saliency(
    attachments: &[LinkGraphAttachment],
    docs_by_id: &HashMap<String, LinkGraphDocument>,
    snapshot: Option<&SaliencySnapshot>,
) -> HashMap<String, VisionAnnotation> {
    if attachments.is_empty() {
        return HashMap::new();
    }

    let mut results = HashMap::new();

    for attachment in attachments {
        if attachment.kind != LinkGraphAttachmentKind::Image {
            continue;
        }

        // Check saliency if snapshot provided
        if let Some(snap) = snapshot {
            if !snap.is_high_saliency(&attachment.attachment_name) {
                continue;
            }
        }

        // TODO: Wire up actual vision analysis
        // For now, return empty annotations
    }

    results
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_noop_provider_returns_none() {
        let provider = NoOpVisionProvider;
        let ingress = VisionIngress::new(Box::new(provider));

        let attachments: Vec<LinkGraphAttachment> = vec![];
        let docs = HashMap::new();

        let rt = tokio::runtime::Runtime::new().unwrap();
        let result = rt.block_on(async { ingress.process_attachments(&attachments, &docs).await });
        assert!(result.is_empty());
    }

    #[test]
    fn test_cross_modal_edges_empty() {
        let annotations = HashMap::new();
        let doc_ids: Vec<String> = Vec::new();

        let edges = build_cross_modal_edges(&annotations, &doc_ids);
        assert!(edges.is_empty());
    }

    #[test]
    fn test_cross_modal_edges_basic() {
        let mut annotations = HashMap::new();
        annotations.insert(
            "image1.png".to_string(),
            VisionAnnotation {
                description: "Rust performance optimization diagram".to_string(),
                confidence: 0.9,
                entities: vec!["rust".to_string(), "performance".to_string()],
                annotated_at: 0,
            },
        );

        let doc_ids = vec!["rust.md".to_string(), "performance.md".to_string()];

        let edges = build_cross_modal_edges(&annotations, &doc_ids);
        assert_eq!(edges.len(), 1);
        assert!(edges.contains_key("image1.png"));
    }

    #[test]
    fn test_process_with_saliency_empty() {
        let attachments: Vec<LinkGraphAttachment> = vec![];
        let docs = HashMap::new();

        let result = process_attachments_with_saliency(&attachments, &docs, None);
        assert!(result.is_empty());
    }
}
