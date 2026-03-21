use std::cmp::Ordering;
use std::collections::HashSet;

use crate::search::fuzzy::{
    FuzzyMatch, FuzzyMatcher, FuzzyScore, FuzzySearchOptions, score_candidate_with_query_chars,
};
use tantivy::collector::TopDocs;
use tantivy::query::QueryParser;
use tantivy::schema::{Field, STORED, STRING, TEXT, Value};
use tantivy::{Index, TantivyDocument, TantivyError};

/// One shared search document stored in Tantivy.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SearchDocument {
    /// Stable identifier used to map search hits back into domain records.
    pub id: String,
    /// Primary title or symbol name.
    pub title: String,
    /// Domain-specific kind label.
    pub kind: String,
    /// Stable path or location for the record.
    pub path: String,
    /// Coarse search scope such as repo or source.
    pub scope: String,
    /// Secondary namespace such as crate or document identifier.
    pub namespace: String,
    /// Additional searchable terms.
    pub terms: Vec<String>,
}

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
    pub fn add_document(&self, document: &SearchDocument) -> Result<(), TantivyError> {
        self.add_documents(std::iter::once(document.clone()))
    }

    /// Add multiple shared search documents in one writer commit.
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

/// One Tantivy-backed fuzzy match.
pub type TantivyDocumentMatch = FuzzyMatch<TantivyDocument>;

/// Shared Tantivy-backed fuzzy matcher for text fields.
pub struct TantivyMatcher<'a> {
    index: &'a Index,
    default_fields: Vec<Field>,
    match_field: Field,
    options: FuzzySearchOptions,
}

impl<'a> TantivyMatcher<'a> {
    /// Create a Tantivy fuzzy matcher for one primary match field.
    #[must_use]
    pub fn new(
        index: &'a Index,
        default_fields: Vec<Field>,
        match_field: Field,
        options: FuzzySearchOptions,
    ) -> Self {
        Self {
            index,
            default_fields,
            match_field,
            options,
        }
    }
}

impl FuzzyMatcher<TantivyDocument> for TantivyMatcher<'_> {
    type Error = TantivyError;

    fn search(&self, query: &str, limit: usize) -> Result<Vec<TantivyDocumentMatch>, Self::Error> {
        let query = query.trim();
        if query.is_empty() || limit == 0 {
            return Ok(Vec::new());
        }
        let mut query_chars = Vec::new();
        let mut candidate_chars = Vec::new();
        let mut scratch = Vec::new();
        let mut seen_ranges = Vec::new();
        let mut boundary_scratch = Vec::new();
        collect_lowercase_chars(query, &mut query_chars);

        let reader = self.index.reader()?;
        let searcher = reader.searcher();

        let mut parser = QueryParser::for_index(self.index, self.default_fields.clone());
        parser.set_field_fuzzy(
            self.match_field,
            false,
            self.options.max_distance.min(2),
            self.options.transposition,
        );
        let query_object = parser.parse_query(query)?;
        let top_docs =
            searcher.search(&query_object, &TopDocs::with_limit(limit.saturating_mul(4)))?;

        let mut matches = Vec::new();
        for (_tantivy_score, doc_address) in top_docs {
            let document: TantivyDocument = searcher.doc(doc_address)?;
            let Some(stored_text) = document
                .get_first(self.match_field)
                .and_then(|value| value.as_str())
                .map(str::to_string)
            else {
                continue;
            };

            let Some((matched_text, score)) = best_match_candidate(
                query,
                query_chars.as_slice(),
                stored_text.as_str(),
                self.options,
                &mut candidate_chars,
                &mut scratch,
                &mut seen_ranges,
                &mut boundary_scratch,
            ) else {
                continue;
            };

            matches.push(FuzzyMatch {
                item: document,
                matched_text,
                score: score.score,
                distance: score.distance,
            });
        }

        matches.sort_by(|left, right| {
            right
                .score
                .partial_cmp(&left.score)
                .unwrap_or(Ordering::Equal)
                .then_with(|| left.distance.cmp(&right.distance))
                .then_with(|| left.matched_text.len().cmp(&right.matched_text.len()))
                .then_with(|| left.matched_text.cmp(&right.matched_text))
        });
        matches.truncate(limit);
        Ok(matches)
    }
}

fn field_string(document: &TantivyDocument, field: Field) -> String {
    document
        .get_first(field)
        .and_then(|value| value.as_str())
        .unwrap_or_default()
        .to_string()
}

fn best_match_candidate(
    query: &str,
    query_chars: &[char],
    stored_text: &str,
    options: FuzzySearchOptions,
    candidate_chars: &mut Vec<char>,
    scratch: &mut Vec<usize>,
    seen_ranges: &mut Vec<(usize, usize)>,
    boundary_scratch: &mut Vec<usize>,
) -> Option<(String, FuzzyScore)> {
    let mut best: Option<(&str, FuzzyScore)> = None;

    for_each_candidate_fragment(stored_text, seen_ranges, boundary_scratch, |candidate| {
        let Some(score) = score_candidate_with_query_chars(
            query,
            query_chars,
            candidate,
            options,
            candidate_chars,
            scratch,
        ) else {
            return;
        };

        let replace = match best.as_ref() {
            None => true,
            Some((best_text, best_score)) => {
                compare_candidate(candidate, score, best_text, *best_score).is_lt()
            }
        };

        if replace {
            best = Some((candidate, score));
        }
    });

    best.map(|(candidate, score)| (candidate.to_string(), score))
}

fn for_each_candidate_fragment<'a>(
    stored_text: &'a str,
    seen_ranges: &mut Vec<(usize, usize)>,
    boundary_scratch: &mut Vec<usize>,
    mut visit: impl FnMut(&'a str),
) {
    seen_ranges.clear();
    push_candidate_fragment(stored_text, stored_text, seen_ranges, &mut visit);
    for fragment in stored_text.split(|ch: char| !ch.is_alphanumeric()) {
        push_candidate_fragment(stored_text, fragment, seen_ranges, &mut visit);
        push_identifier_subfragments(
            stored_text,
            fragment,
            seen_ranges,
            boundary_scratch,
            &mut visit,
        );
    }
}

fn push_candidate_fragment<'a>(
    stored_text: &'a str,
    fragment: &'a str,
    seen_ranges: &mut Vec<(usize, usize)>,
    visit: &mut impl FnMut(&'a str),
) {
    let fragment = fragment.trim();
    if fragment.is_empty() {
        return;
    }
    if seen_ranges
        .iter()
        .any(|&(start, end)| fragment_eq_ignore_case(&stored_text[start..end], fragment))
    {
        return;
    }
    seen_ranges.push(byte_range_in_parent(stored_text, fragment));
    visit(fragment);
}

fn push_identifier_subfragments<'a>(
    stored_text: &'a str,
    fragment: &'a str,
    seen_ranges: &mut Vec<(usize, usize)>,
    boundary_scratch: &mut Vec<usize>,
    visit: &mut impl FnMut(&'a str),
) {
    populate_identifier_boundaries(fragment, boundary_scratch);
    if boundary_scratch.len() <= 2 {
        return;
    }

    for start_idx in 0..(boundary_scratch.len() - 1) {
        for end_idx in (start_idx + 1)..boundary_scratch.len() {
            let start = boundary_scratch[start_idx];
            let end = boundary_scratch[end_idx];
            if start == 0 && end == fragment.len() {
                continue;
            }
            push_candidate_fragment(stored_text, &fragment[start..end], seen_ranges, visit);
        }
    }
}

fn populate_identifier_boundaries(fragment: &str, boundaries: &mut Vec<usize>) {
    boundaries.clear();
    let mut chars = fragment.char_indices().peekable();
    let Some((_, mut prev)) = chars.next() else {
        boundaries.push(0);
        return;
    };

    boundaries.push(0);
    while let Some((byte_idx, ch)) = chars.next() {
        let next = chars.peek().map(|(_, next)| *next);

        let lower_to_upper = prev.is_lowercase() && ch.is_uppercase();
        let acronym_to_word =
            prev.is_uppercase() && ch.is_uppercase() && next.is_some_and(char::is_lowercase);
        let alpha_to_digit = prev.is_alphabetic() && ch.is_ascii_digit();
        let digit_to_alpha = prev.is_ascii_digit() && ch.is_alphabetic();

        if lower_to_upper || acronym_to_word || alpha_to_digit || digit_to_alpha {
            boundaries.push(byte_idx);
        }
        prev = ch;
    }
    boundaries.push(fragment.len());
}

fn byte_range_in_parent(parent: &str, fragment: &str) -> (usize, usize) {
    let start = fragment.as_ptr() as usize - parent.as_ptr() as usize;
    (start, start + fragment.len())
}

fn fragment_eq_ignore_case(left: &str, right: &str) -> bool {
    left.chars()
        .flat_map(char::to_lowercase)
        .eq(right.chars().flat_map(char::to_lowercase))
}

fn compare_candidate(
    left_text: &str,
    left_score: FuzzyScore,
    right_text: &str,
    right_score: FuzzyScore,
) -> Ordering {
    right_score
        .score
        .partial_cmp(&left_score.score)
        .unwrap_or(Ordering::Equal)
        .then_with(|| left_score.distance.cmp(&right_score.distance))
        .then_with(|| left_text.len().cmp(&right_text.len()))
        .then_with(|| left_text.cmp(right_text))
}

fn collect_lowercase_chars(value: &str, target: &mut Vec<char>) {
    target.clear();
    target.extend(value.chars().flat_map(char::to_lowercase));
}

#[cfg(test)]
mod tests {
    use super::*;

    fn adjacent_identifier_fragments(value: &str) -> Vec<&str> {
        let mut boundaries = Vec::new();
        populate_identifier_boundaries(value, &mut boundaries);
        boundaries
            .windows(2)
            .map(|range| &value[range[0]..range[1]])
            .collect()
    }

    #[test]
    fn search_document_index_supports_exact_lookup() {
        let index = SearchDocumentIndex::new();
        index
            .add_documents(vec![SearchDocument {
                id: "page:1".to_string(),
                title: "Solve Linear Systems".to_string(),
                kind: "reference".to_string(),
                path: "docs/solve.md".to_string(),
                scope: "repo".to_string(),
                namespace: "solve-guide".to_string(),
                terms: vec!["solver".to_string(), "matrix".to_string()],
            }])
            .expect("shared document indexing succeeds");

        let results = index
            .search_exact("solver", 10)
            .expect("exact search succeeds");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].id, "page:1");
    }

    #[test]
    fn tantivy_matcher_uses_best_fragment_for_fuzzy_titles() {
        let index = SearchDocumentIndex::new();
        index
            .add_documents(vec![SearchDocument {
                id: "page:1".to_string(),
                title: "Solve Linear Systems".to_string(),
                kind: "reference".to_string(),
                path: "docs/solve.md".to_string(),
                scope: "repo".to_string(),
                namespace: "solve-guide".to_string(),
                terms: vec!["solver".to_string()],
            }])
            .expect("shared document indexing succeeds");

        let results = index
            .search_fuzzy("slove", 10, FuzzySearchOptions::document_search())
            .expect("fuzzy search succeeds");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].item.id, "page:1");
        assert_eq!(results[0].matched_text, "Solve");
    }

    #[test]
    fn candidate_fragments_split_camel_case_and_identifier_spans() {
        let mut seen_ranges = Vec::new();
        let mut boundary_scratch = Vec::new();
        let mut fragments = Vec::new();
        for_each_candidate_fragment(
            "solveLinearSystem2D",
            &mut seen_ranges,
            &mut boundary_scratch,
            |fragment| fragments.push(fragment.to_string()),
        );
        assert!(fragments.iter().any(|fragment| fragment == "solve"));
        assert!(fragments.iter().any(|fragment| fragment == "Linear"));
        assert!(fragments.iter().any(|fragment| fragment == "System"));
        assert!(fragments.iter().any(|fragment| fragment == "2"));
        assert!(fragments.iter().any(|fragment| fragment == "D"));
        assert!(fragments.iter().any(|fragment| fragment == "LinearSystem"));
    }

    #[test]
    fn populate_identifier_boundaries_tracks_camel_case_and_digit_edges() {
        let fragments = adjacent_identifier_fragments("solveLinearSystem2D");
        assert_eq!(fragments, vec!["solve", "Linear", "System", "2", "D"]);
    }

    #[test]
    fn populate_identifier_boundaries_tracks_acronym_to_word_edges() {
        let fragments = adjacent_identifier_fragments("HTTPRequest");
        assert_eq!(fragments, vec!["HTTP", "Request"]);
    }

    #[test]
    fn candidate_fragments_deduplicate_case_insensitive_repeats() {
        let mut seen_ranges = Vec::new();
        let mut boundary_scratch = Vec::new();
        let mut fragments = Vec::new();
        for_each_candidate_fragment(
            "Solve solve SOLVE",
            &mut seen_ranges,
            &mut boundary_scratch,
            |fragment| fragments.push(fragment.to_string()),
        );

        let solve_count = fragments
            .iter()
            .filter(|fragment| fragment.eq_ignore_ascii_case("solve"))
            .count();
        assert_eq!(solve_count, 1);
    }

    #[test]
    fn best_match_candidate_prefers_camel_case_subfragments() {
        let mut query_chars = Vec::new();
        let mut candidate_chars = Vec::new();
        let mut scratch = Vec::new();
        let mut seen_ranges = Vec::new();
        let mut boundary_scratch = Vec::new();
        collect_lowercase_chars("equations", &mut query_chars);
        let best = best_match_candidate(
            "equations",
            query_chars.as_slice(),
            "DifferentialEquations",
            FuzzySearchOptions::document_search(),
            &mut candidate_chars,
            &mut scratch,
            &mut seen_ranges,
            &mut boundary_scratch,
        )
        .expect("best fragment should be found");
        assert_eq!(best.0, "Equations");
        assert_eq!(best.1.distance, 0);
    }
}
