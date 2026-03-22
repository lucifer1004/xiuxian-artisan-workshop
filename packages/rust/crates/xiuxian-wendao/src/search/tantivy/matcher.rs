use std::cmp::Ordering;

use crate::search::fuzzy::{FuzzyMatch, FuzzyMatcher, FuzzySearchOptions};
use tantivy::collector::TopDocs;
use tantivy::query::QueryParser;
use tantivy::schema::{Field, Value};
use tantivy::{Index, TantivyDocument, TantivyError};

use super::compare::{best_match_candidate, collect_lowercase_chars};

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

        matches.sort_by(compare_tantivy_matches);
        matches.truncate(limit);
        Ok(matches)
    }
}

fn compare_tantivy_matches(left: &TantivyDocumentMatch, right: &TantivyDocumentMatch) -> Ordering {
    right
        .score
        .partial_cmp(&left.score)
        .unwrap_or(Ordering::Equal)
        .then_with(|| left.distance.cmp(&right.distance))
        .then_with(|| left.matched_text.len().cmp(&right.matched_text.len()))
        .then_with(|| left.matched_text.cmp(&right.matched_text))
}
