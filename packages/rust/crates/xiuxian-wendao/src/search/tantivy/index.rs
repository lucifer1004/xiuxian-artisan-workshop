use std::collections::HashSet;

use crate::search::fuzzy::{FuzzyMatch, FuzzySearchOptions};
use tantivy::collector::TopDocs;
use tantivy::query::{
    BooleanQuery, BoostQuery, Occur, PhrasePrefixQuery, Query, QueryParser, TermQuery,
};
use tantivy::schema::IndexRecordOption;
use tantivy::{Index, IndexReader, ReloadPolicy, TantivyDocument, TantivyError, Term};

use super::document::{SearchDocument, SearchDocumentHit};
use super::fields::SearchDocumentFields;
use super::matcher::TantivyMatcher;
use super::tokenizer::{collect_search_tokens, register_search_tokenizer};

const SEARCH_CANDIDATE_WINDOW_CAP: usize = 96;
const SEARCH_CANDIDATE_WINDOW_MULTIPLIER: usize = 3;

/// Shared Tantivy-backed index for domain-agnostic search documents.
#[derive(Clone)]
pub struct SearchDocumentIndex {
    /// In-memory Tantivy index.
    pub index: Index,
    /// Shared document fields.
    pub fields: SearchDocumentFields,
    reader: IndexReader,
}

impl SearchDocumentIndex {
    /// Create an empty in-memory search index using the shared schema.
    ///
    /// # Panics
    ///
    /// Panics when Tantivy cannot initialize the shared in-memory reader.
    #[must_use]
    pub fn new() -> Self {
        let fields = SearchDocumentFields::new();
        let index = Index::create_in_ram(fields.schema.clone());
        register_search_tokenizer(&index);
        let reader = index
            .reader_builder()
            .reload_policy(ReloadPolicy::Manual)
            .try_into()
            .unwrap_or_else(|error| panic!("shared Tantivy reader should initialize: {error}"));
        Self {
            index,
            fields,
            reader,
        }
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
        self.reader.reload()?;
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

        let searcher = self.reader.searcher();
        let query = self.build_exact_query(query)?;
        let top_docs = searcher.search(&*query, &TopDocs::with_limit(candidate_limit(limit)))?;

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

    /// Run an exact query and return lightweight hit metadata for caller-side rehydration.
    ///
    /// # Errors
    ///
    /// Returns an error when Tantivy cannot parse or execute the query.
    pub fn search_exact_hits(
        &self,
        query: &str,
        limit: usize,
    ) -> Result<Vec<SearchDocumentHit>, TantivyError> {
        let query = query.trim();
        if query.is_empty() || limit == 0 {
            return Ok(Vec::new());
        }

        let searcher = self.reader.searcher();
        let query_object = self.build_exact_query(query)?;
        let top_docs =
            searcher.search(&*query_object, &TopDocs::with_limit(candidate_limit(limit)))?;
        self.collect_hits(&searcher, top_docs, limit)
    }

    /// Run a phrase-prefix query over the shared search fields.
    ///
    /// # Errors
    ///
    /// Returns an error when Tantivy cannot execute the prefix query.
    pub fn search_prefix_hits(
        &self,
        query: &str,
        limit: usize,
    ) -> Result<Vec<SearchDocumentHit>, TantivyError> {
        let query = query.trim();
        if query.is_empty() || limit == 0 {
            return Ok(Vec::new());
        }
        let Some(query_object) = self.build_phrase_prefix_query(query) else {
            return Ok(Vec::new());
        };

        let searcher = self.reader.searcher();
        let top_docs =
            searcher.search(&*query_object, &TopDocs::with_limit(candidate_limit(limit)))?;
        self.collect_hits(&searcher, top_docs, limit)
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
            self.fields.text_fields(),
            self.fields.match_field_specs().to_vec(),
            options,
        );
        let raw_matches = matcher.search_with_fields(query, limit)?;

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

    /// Run a fuzzy query and return lightweight hit metadata for caller-side rehydration.
    ///
    /// # Errors
    ///
    /// Returns an error when Tantivy cannot execute the fuzzy query.
    pub fn search_fuzzy_hits(
        &self,
        query: &str,
        limit: usize,
        options: FuzzySearchOptions,
    ) -> Result<Vec<SearchDocumentHit>, TantivyError> {
        let query = query.trim();
        if query.is_empty() || limit == 0 {
            return Ok(Vec::new());
        }

        let matcher = TantivyMatcher::new(
            &self.index,
            self.fields.text_fields(),
            self.fields.match_field_specs().to_vec(),
            options,
        );
        let raw_matches = matcher.search_with_fields(query, limit)?;
        let mut hits = Vec::new();
        let mut seen_ids = HashSet::new();

        for raw_match in raw_matches {
            let hit = self.fields.parse_hit(
                &raw_match.item,
                raw_match.score,
                raw_match.matched_field,
                Some(raw_match.matched_text),
                raw_match.distance,
            );
            if !seen_ids.insert(hit.id.clone()) {
                continue;
            }
            hits.push(hit);
            if hits.len() >= limit {
                break;
            }
        }

        Ok(hits)
    }

    fn build_exact_query(&self, query: &str) -> Result<Box<dyn Query>, TantivyError> {
        let normalized_query = normalize_exact_query(query);
        let mut clauses: Vec<(Occur, Box<dyn Query>)> = Vec::new();

        for spec in self.fields.match_field_specs() {
            if let Some(exact_field) = spec.exact_field {
                let term = Term::from_field_text(exact_field, normalized_query.as_str());
                clauses.push((
                    Occur::Should,
                    Box::new(BoostQuery::new(
                        Box::new(TermQuery::new(term, IndexRecordOption::Basic)),
                        spec.query_boost,
                    )),
                ));
            }
        }

        let parser = QueryParser::for_index(&self.index, self.fields.text_fields());
        clauses.push((Occur::Should, parser.parse_query(query)?));
        Ok(Box::new(BooleanQuery::new(clauses)))
    }

    fn build_phrase_prefix_query(&self, query: &str) -> Option<Box<dyn Query>> {
        let tokens = collect_search_tokens(&self.index, query);
        if tokens.len() < 2 {
            return None;
        }

        let clauses = self
            .fields
            .match_field_specs()
            .into_iter()
            .map(|spec| {
                let terms = tokens
                    .iter()
                    .map(|token| Term::from_field_text(spec.text_field, token))
                    .collect::<Vec<_>>();
                (
                    Occur::Should,
                    Box::new(BoostQuery::new(
                        Box::new(PhrasePrefixQuery::new(terms)),
                        spec.query_boost,
                    )) as Box<dyn Query>,
                )
            })
            .collect::<Vec<_>>();
        Some(Box::new(BooleanQuery::new(clauses)))
    }

    fn collect_hits(
        &self,
        searcher: &tantivy::Searcher,
        top_docs: Vec<(f32, tantivy::DocAddress)>,
        limit: usize,
    ) -> Result<Vec<SearchDocumentHit>, TantivyError> {
        let mut hits = Vec::new();
        let mut seen_ids = HashSet::new();
        for (score, doc_address) in top_docs {
            let document: TantivyDocument = searcher.doc(doc_address)?;
            let hit = self.fields.parse_hit(&document, score, None, None, 0);
            if !seen_ids.insert(hit.id.clone()) {
                continue;
            }
            hits.push(hit);
            if hits.len() >= limit {
                break;
            }
        }
        Ok(hits)
    }
}

impl Default for SearchDocumentIndex {
    fn default() -> Self {
        Self::new()
    }
}

fn candidate_limit(limit: usize) -> usize {
    limit
        .max(1)
        .saturating_mul(SEARCH_CANDIDATE_WINDOW_MULTIPLIER)
        .min(SEARCH_CANDIDATE_WINDOW_CAP)
}

fn normalize_exact_query(query: &str) -> String {
    query.trim().chars().flat_map(char::to_lowercase).collect()
}
