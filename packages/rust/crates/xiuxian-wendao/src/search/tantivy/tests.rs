use crate::search::fuzzy::FuzzySearchOptions;

use super::compare::{best_match_candidate, collect_lowercase_chars};
use super::fragments::for_each_candidate_fragment;
use super::identifier::populate_identifier_boundaries;
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
        .unwrap_or_else(|error| panic!("shared document indexing succeeds: {error}"));

    let results = index
        .search_exact("solver", 10)
        .unwrap_or_else(|error| panic!("exact search succeeds: {error}"));
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
        .unwrap_or_else(|error| panic!("shared document indexing succeeds: {error}"));

    let results = index
        .search_fuzzy("slove", 10, FuzzySearchOptions::document_search())
        .unwrap_or_else(|error| panic!("fuzzy search succeeds: {error}"));
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
    .unwrap_or_else(|| panic!("best fragment should be found"));
    assert_eq!(best.0, "Equations");
    assert_eq!(best.1.distance, 0);
}
