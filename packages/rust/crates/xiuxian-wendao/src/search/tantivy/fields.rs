use std::collections::HashSet;

use tantivy::TantivyDocument;
use tantivy::schema::{Field, STORED, STRING, TEXT, Value};

use super::document::SearchDocument;

/// Shared field set for common Tantivy-backed search documents.
#[derive(Debug, Clone)]
pub struct SearchDocumentFields {
    /// Tantivy schema built from the shared field set.
    pub schema: tantivy::schema::Schema,
    /// Stable record identifier.
    pub id: Field,
    /// Primary title or name field.
    pub title: Field,
    /// Domain-specific kind field.
    pub kind: Field,
    /// Stable path or location field.
    pub path: Field,
    /// Coarse scope field such as repo or source.
    pub scope: Field,
    /// Secondary namespace field such as crate name or doc identifier.
    pub namespace: Field,
    /// Additional queryable terms.
    pub terms: Field,
}

impl SearchDocumentFields {
    /// Build the shared search schema.
    #[must_use]
    pub fn new() -> Self {
        let mut schema_builder = tantivy::schema::Schema::builder();
        let id = schema_builder.add_text_field("id", STRING | STORED);
        let title = schema_builder.add_text_field("title", TEXT | STORED);
        let kind = schema_builder.add_text_field("kind", STRING | STORED);
        let path = schema_builder.add_text_field("path", TEXT | STORED);
        let scope = schema_builder.add_text_field("scope", STRING | STORED);
        let namespace = schema_builder.add_text_field("namespace", TEXT | STORED);
        let terms = schema_builder.add_text_field("terms", TEXT | STORED);

        Self {
            schema: schema_builder.build(),
            id,
            title,
            kind,
            path,
            scope,
            namespace,
            terms,
        }
    }

    /// Default fields used for exact lookup.
    #[must_use]
    pub fn default_fields(&self) -> Vec<Field> {
        vec![self.title, self.namespace, self.path, self.terms]
    }

    /// Build one Tantivy document from a shared record.
    #[must_use]
    pub fn make_document(&self, record: &SearchDocument) -> TantivyDocument {
        let mut document = TantivyDocument::default();
        document.add_text(self.id, &record.id);
        document.add_text(self.title, &record.title);
        document.add_text(self.kind, &record.kind);
        document.add_text(self.path, &record.path);
        document.add_text(self.scope, &record.scope);
        document.add_text(self.namespace, &record.namespace);

        let mut seen_terms = HashSet::new();
        for term in &record.terms {
            let normalized = term.trim();
            if normalized.is_empty() || !seen_terms.insert(normalized.to_ascii_lowercase()) {
                continue;
            }
            document.add_text(self.terms, normalized);
        }

        document
    }

    /// Parse one shared record from a Tantivy document.
    #[must_use]
    pub fn parse_document(&self, document: &TantivyDocument) -> SearchDocument {
        let mut terms = document
            .get_all(self.terms)
            .filter_map(|value| value.as_str())
            .map(str::to_string)
            .collect::<Vec<_>>();
        terms.sort();
        terms.dedup();

        SearchDocument {
            id: field_string(document, self.id),
            title: field_string(document, self.title),
            kind: field_string(document, self.kind),
            path: field_string(document, self.path),
            scope: field_string(document, self.scope),
            namespace: field_string(document, self.namespace),
            terms,
        }
    }
}

impl Default for SearchDocumentFields {
    fn default() -> Self {
        Self::new()
    }
}

fn field_string(document: &TantivyDocument, field: Field) -> String {
    document
        .get_first(field)
        .and_then(|value| value.as_str())
        .unwrap_or_default()
        .to_string()
}
