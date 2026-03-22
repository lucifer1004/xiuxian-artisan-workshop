use std::collections::HashSet;

use crate::search::fuzzy::{FuzzyMatch, FuzzyMatcher, FuzzySearchOptions};
use tantivy::collector::TopDocs;
use tantivy::query::QueryParser;
use tantivy::{Index, TantivyDocument, TantivyError};

use super::document::SearchDocument;
use super::fields::SearchDocumentFields;
use super::matcher::TantivyMatcher;

/// Shared Tantivy-backed index for domain-agnostic search documents.
#[derive(Debug, Clone)]
pub struct SearchDocumentIndex {
    /// In-memory Tantivy index.
    pub index: Index,
    /// Shared document fields.
    pub fields: SearchDocumentFields,
}

impl SearchDocumentIndex {
    /// Create an empty in-memory search index using the shared schema.
    #[must_use]
    pub fn new() -> Self {
        let fields = SearchDocumentFields::new();
        let index = Index::create_in_ram(fields.schema.clone());
        Self { index, fields }
    }

    /// Add a single shared search document to the index.
    ///
    /// # Errors
    ///
    /// Returns an error when Tantivy cannot write or commit the document.
    pub fn add_document(&self, document: &SearchDocument) -> Result<(), TantivyError> {
        self.add_documents(std::iter::once(document.clone()))
    }

    /// Add multiple shared search documents in one writer commit.
    ///
    /// # Errors
    ///
    /// Returns an error when Tantivy cannot open a writer or commit the batch.
    pub fn add_documents<I>(&self, documents: I) -> Result<(), TantivyError>
    where
        I: IntoIterator<Item = SearchDocument>,
    {
        let mut writer = self.index.writer(50_000_000)?;
        for document in documents {
            let _ = writer.add_document(self.fields.make_document(&document));
        }
        let _ = writer.commit()?;
        Ok(())
    }

    /// Run an exact query over the shared search fields.
    ///
    /// # Errors
    ///
    /// Returns an error when Tantivy cannot parse or execute the query.
    pub fn search_exact(
        &self,
        query: &str,
        limit: usize,
    ) -> Result<Vec<SearchDocument>, TantivyError> {
        let query = query.trim();
        if query.is_empty() || limit == 0 {
            return Ok(Vec::new());
        }

        let reader = self.index.reader()?;
        let searcher = reader.searcher();
        let parser = QueryParser::for_index(&self.index, self.fields.default_fields());
        let query = parser.parse_query(query)?;
        let top_docs = searcher.search(&query, &TopDocs::with_limit(limit.saturating_mul(4)))?;

        let mut records = Vec::new();
        let mut seen_ids = HashSet::new();
        for (_score, doc_address) in top_docs {
            let document: TantivyDocument = searcher.doc(doc_address)?;
            let record = self.fields.parse_document(&document);
            if !seen_ids.insert(record.id.clone()) {
                continue;
            }
            records.push(record);
            if records.len() >= limit {
                break;
            }
        }

        Ok(records)
    }

    /// Run a fuzzy query over the shared search fields.
    ///
    /// # Errors
    ///
    /// Returns an error when Tantivy cannot execute the fuzzy query.
    pub fn search_fuzzy(
        &self,
        query: &str,
        limit: usize,
        options: FuzzySearchOptions,
    ) -> Result<Vec<FuzzyMatch<SearchDocument>>, TantivyError> {
        let query = query.trim();
        if query.is_empty() || limit == 0 {
            return Ok(Vec::new());
        }

        let matcher = TantivyMatcher::new(
            &self.index,
            self.fields.default_fields(),
            self.fields.title,
            options,
        );
        let raw_matches = matcher.search(query, limit.saturating_mul(4))?;

        let mut records = Vec::new();
        let mut seen_ids = HashSet::new();
        for raw_match in raw_matches {
            let record = self.fields.parse_document(&raw_match.item);
            if !seen_ids.insert(record.id.clone()) {
                continue;
            }
            records.push(FuzzyMatch {
                item: record,
                matched_text: raw_match.matched_text,
                score: raw_match.score,
                distance: raw_match.distance,
            });
            if records.len() >= limit {
                break;
            }
        }

        Ok(records)
    }
}

impl Default for SearchDocumentIndex {
    fn default() -> Self {
        Self::new()
    }
}
