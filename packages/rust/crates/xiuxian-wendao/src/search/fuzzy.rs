use std::cmp::Ordering;
use std::convert::Infallible;
use std::{cell::RefCell, thread_local};

/// Shared fuzzy-search configuration.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FuzzySearchOptions {
    /// Maximum edit distance allowed for a match.
    pub max_distance: u8,
    /// Number of leading characters that must match exactly.
    pub prefix_length: usize,
    /// Whether adjacent transpositions count as one edit.
    pub transposition: bool,
}

impl FuzzySearchOptions {
    /// Create a new fuzzy-search configuration.
    #[must_use]
    pub const fn new(max_distance: u8, prefix_length: usize, transposition: bool) -> Self {
        Self {
            max_distance,
            prefix_length,
            transposition,
        }
    }

    /// Default option profile for symbol-like identifiers.
    #[must_use]
    pub const fn symbol_search() -> Self {
        Self::new(1, 1, true)
    }

    /// Default option profile for CamelCase-style symbol abbreviations.
    #[must_use]
    pub const fn camel_case_symbol() -> Self {
        Self::new(1, 0, true)
    }

    /// Default option profile for document-like titles and phrases.
    #[must_use]
    pub const fn document_search() -> Self {
        Self::new(2, 0, true)
    }

    /// Default option profile for path/title discovery.
    #[must_use]
    pub const fn path_search() -> Self {
        Self::document_search()
    }
}

impl Default for FuzzySearchOptions {
    fn default() -> Self {
        Self::symbol_search()
    }
}

/// One scored fuzzy match result.
#[derive(Debug, Clone)]
pub struct FuzzyMatch<T> {
    /// Matched item.
    pub item: T,
    /// The candidate text used for matching.
    pub matched_text: String,
    /// Normalized similarity score in `[0.0, 1.0]`.
    pub score: f32,
    /// Edit distance between query and candidate text.
    pub distance: usize,
}

/// A compact score result for one fuzzy comparison.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FuzzyScore {
    /// Normalized similarity score in `[0.0, 1.0]`.
    pub score: f32,
    /// Edit distance between the compared strings.
    pub distance: usize,
}

/// Shared fuzzy matcher abstraction.
pub trait FuzzyMatcher<T> {
    /// Error type emitted by the matcher.
    type Error;

    /// Search for fuzzy matches for one query.
    fn search(&self, query: &str, limit: usize) -> Result<Vec<FuzzyMatch<T>>, Self::Error>;
}

/// Generic lexical matcher over an in-memory candidate slice.
pub struct LexicalMatcher<'a, T, F> {
    candidates: &'a [T],
    extract: F,
    options: FuzzySearchOptions,
}

impl<'a, T, F> LexicalMatcher<'a, T, F> {
    /// Create a lexical matcher.
    #[must_use]
    pub fn new(candidates: &'a [T], extract: F, options: FuzzySearchOptions) -> Self {
        Self {
            candidates,
            extract,
            options,
        }
    }

    /// Access the matcher options.
    #[must_use]
    pub const fn options(&self) -> FuzzySearchOptions {
        self.options
    }
}

impl<'a, T, F> FuzzyMatcher<T> for LexicalMatcher<'a, T, F>
where
    T: Clone,
    F: for<'b> Fn(&'b T) -> &'b str,
{
    type Error = Infallible;

    fn search(&self, query: &str, limit: usize) -> Result<Vec<FuzzyMatch<T>>, Self::Error> {
        let query = query.trim();
        if query.is_empty() || limit == 0 {
            return Ok(Vec::new());
        }
        let mut matches = with_thread_local_buffers(|buffers| {
            collect_lowercase_chars(query, &mut buffers.left_chars);

            let mut matches = Vec::new();
            for candidate in self.candidates {
                let matched_text = (self.extract)(candidate);
                if let Some(score) = score_candidate_with_query_chars(
                    query,
                    buffers.left_chars.as_slice(),
                    matched_text,
                    self.options,
                    &mut buffers.right_chars,
                    &mut buffers.distance_scratch,
                ) {
                    matches.push(FuzzyMatch {
                        item: candidate.clone(),
                        matched_text: matched_text.to_string(),
                        score: score.score,
                        distance: score.distance,
                    });
                }
            }

            matches
        });

        matches.sort_by(compare_fuzzy_matches);
        matches.truncate(limit);
        Ok(matches)
    }
}

/// Compute the shared-prefix length in Unicode scalar values.
#[must_use]
pub fn shared_prefix_len(left: &str, right: &str) -> usize {
    left.chars()
        .zip(right.chars())
        .take_while(|(left, right)| chars_equal_ignore_case(*left, *right))
        .count()
}

/// Check whether a candidate satisfies the prefix-length requirement.
#[must_use]
pub fn passes_prefix_requirement(query: &str, candidate: &str, prefix_length: usize) -> bool {
    if prefix_length == 0 {
        return true;
    }
    shared_prefix_len(query, candidate) >= prefix_length
}

/// Calculate classic Levenshtein distance without transposition support.
#[must_use]
pub fn levenshtein_distance(left: &str, right: &str) -> usize {
    edit_distance(left, right, false)
}

/// Calculate edit distance, optionally treating adjacent transpositions as one edit.
#[must_use]
pub fn edit_distance(left: &str, right: &str, transposition: bool) -> usize {
    with_thread_local_buffers(|buffers| {
        collect_chars(left, &mut buffers.left_chars);
        collect_chars(right, &mut buffers.right_chars);
        edit_distance_with_scratch(
            buffers.left_chars.as_slice(),
            buffers.right_chars.as_slice(),
            transposition,
            &mut buffers.distance_scratch,
        )
    })
}

fn edit_distance_with_scratch(
    left_chars: &[char],
    right_chars: &[char],
    transposition: bool,
    scratch: &mut Vec<usize>,
) -> usize {
    let left_len = left_chars.len();
    let right_len = right_chars.len();

    if left_len == 0 {
        return right_len;
    }
    if right_len == 0 {
        return left_len;
    }

    let row_len = right_len + 1;
    scratch.clear();
    scratch.resize(row_len.saturating_mul(3), 0);
    let (prev_prev_row, tail) = scratch.split_at_mut(row_len);
    let (prev_row, curr_row) = tail.split_at_mut(row_len);

    for (col_idx, cell) in prev_row.iter_mut().enumerate() {
        *cell = col_idx;
    }

    for left_idx in 1..=left_len {
        curr_row[0] = left_idx;
        for right_idx in 1..=right_len {
            let cost = usize::from(left_chars[left_idx - 1] != right_chars[right_idx - 1]);
            let deletion = prev_row[right_idx] + 1;
            let insertion = curr_row[right_idx - 1] + 1;
            let substitution = prev_row[right_idx - 1] + cost;
            let mut best = deletion.min(insertion).min(substitution);

            if transposition
                && left_idx > 1
                && right_idx > 1
                && left_chars[left_idx - 1] == right_chars[right_idx - 2]
                && left_chars[left_idx - 2] == right_chars[right_idx - 1]
            {
                best = best.min(prev_prev_row[right_idx - 2] + 1);
            }

            curr_row[right_idx] = best;
        }
        prev_prev_row.copy_from_slice(prev_row);
        prev_row.copy_from_slice(curr_row);
    }

    prev_row[right_len]
}

/// Calculate normalized similarity score from edit distance.
#[must_use]
pub fn normalized_score(left: &str, right: &str, transposition: bool) -> f32 {
    with_thread_local_buffers(|buffers| {
        collect_chars(left, &mut buffers.left_chars);
        collect_chars(right, &mut buffers.right_chars);
        score_from_char_slices(
            buffers.left_chars.as_slice(),
            buffers.right_chars.as_slice(),
            transposition,
            &mut buffers.distance_scratch,
        )
    })
}

/// Score one candidate against one query using the shared options.
#[must_use]
pub fn score_candidate(
    query: &str,
    candidate: &str,
    options: FuzzySearchOptions,
) -> Option<FuzzyScore> {
    with_thread_local_buffers(|buffers| {
        collect_lowercase_chars(query, &mut buffers.left_chars);
        score_candidate_with_query_chars(
            query,
            buffers.left_chars.as_slice(),
            candidate,
            options,
            &mut buffers.right_chars,
            &mut buffers.distance_scratch,
        )
    })
}

pub(crate) fn score_candidate_with_query_chars(
    query: &str,
    query_chars: &[char],
    candidate: &str,
    options: FuzzySearchOptions,
    candidate_chars: &mut Vec<char>,
    scratch: &mut Vec<usize>,
) -> Option<FuzzyScore> {
    let shared_prefix =
        collect_lowercase_chars_and_shared_prefix(query, candidate, candidate_chars);
    if options.prefix_length > 0 && shared_prefix < options.prefix_length {
        return None;
    }

    score_candidate_from_char_slices(
        query_chars,
        candidate_chars.as_slice(),
        options.transposition,
        options.max_distance,
        scratch,
    )
}

fn normalized_score_from_distance(distance: usize, max_len: usize) -> f32 {
    1.0 - bounded_ratio(distance, max_len)
}

fn score_from_char_slices(
    left_chars: &[char],
    right_chars: &[char],
    transposition: bool,
    scratch: &mut Vec<usize>,
) -> f32 {
    let max_len = left_chars.len().max(right_chars.len());
    if max_len == 0 {
        return 1.0;
    }

    let distance = edit_distance_with_scratch(left_chars, right_chars, transposition, scratch);
    normalized_score_from_distance(distance, max_len)
}

fn score_candidate_from_char_slices(
    query_chars: &[char],
    candidate_chars: &[char],
    transposition: bool,
    max_distance: u8,
    scratch: &mut Vec<usize>,
) -> Option<FuzzyScore> {
    let max_len = query_chars.len().max(candidate_chars.len());
    let distance = edit_distance_with_scratch(query_chars, candidate_chars, transposition, scratch);
    if distance > usize::from(max_distance) {
        return None;
    }

    Some(FuzzyScore {
        score: if max_len == 0 {
            1.0
        } else {
            normalized_score_from_distance(distance, max_len)
        },
        distance,
    })
}

#[derive(Default)]
struct FuzzyThreadLocalBuffers {
    left_chars: Vec<char>,
    right_chars: Vec<char>,
    distance_scratch: Vec<usize>,
}

thread_local! {
    static FUZZY_THREAD_LOCAL_BUFFERS: RefCell<FuzzyThreadLocalBuffers> =
        RefCell::new(FuzzyThreadLocalBuffers::default());
}

fn with_thread_local_buffers<T>(operation: impl FnOnce(&mut FuzzyThreadLocalBuffers) -> T) -> T {
    FUZZY_THREAD_LOCAL_BUFFERS.with(|buffers| {
        let mut buffers = buffers.borrow_mut();
        operation(&mut buffers)
    })
}

fn collect_chars(value: &str, target: &mut Vec<char>) {
    target.clear();
    target.extend(value.chars());
}

fn collect_lowercase_chars(value: &str, target: &mut Vec<char>) {
    target.clear();
    target.extend(value.chars().flat_map(char::to_lowercase));
}

fn collect_lowercase_chars_and_shared_prefix(
    query: &str,
    candidate: &str,
    target: &mut Vec<char>,
) -> usize {
    let mut query_chars = query.chars();
    let mut shared_prefix = 0;
    let mut prefix_matches = true;

    target.clear();
    for candidate_char in candidate.chars() {
        if prefix_matches {
            match query_chars.next() {
                Some(query_char) if chars_equal_ignore_case(query_char, candidate_char) => {
                    shared_prefix += 1;
                }
                _ => {
                    prefix_matches = false;
                }
            }
        }
        target.extend(candidate_char.to_lowercase());
    }

    shared_prefix
}

fn chars_equal_ignore_case(left: char, right: char) -> bool {
    left.to_lowercase().eq(right.to_lowercase())
}

fn compare_fuzzy_matches<T>(left: &FuzzyMatch<T>, right: &FuzzyMatch<T>) -> Ordering {
    right
        .score
        .partial_cmp(&left.score)
        .unwrap_or(Ordering::Equal)
        .then_with(|| left.distance.cmp(&right.distance))
        .then_with(|| left.matched_text.len().cmp(&right.matched_text.len()))
        .then_with(|| left.matched_text.cmp(&right.matched_text))
}

fn bounded_ratio(numerator: usize, denominator: usize) -> f32 {
    let numerator = bounded_usize_to_f32(numerator);
    let denominator = bounded_usize_to_f32(denominator.max(1));
    numerator / denominator
}

fn bounded_usize_to_f32(value: usize) -> f32 {
    u16::try_from(value).map_or(f32::from(u16::MAX), f32::from)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn levenshtein_distance_keeps_transposition_as_two_without_flag() {
        assert_eq!(levenshtein_distance("storage", "stroage"), 2);
    }

    #[test]
    fn edit_distance_supports_transposition_when_enabled() {
        assert_eq!(edit_distance("storage", "stroage", true), 1);
    }

    #[test]
    fn lexical_matcher_respects_prefix_requirement() {
        let candidates = vec!["spawn".to_string(), "plan".to_string()];
        fn string_value(candidate: &String) -> &str {
            candidate.as_str()
        }

        let matcher = LexicalMatcher::new(
            &candidates,
            string_value,
            FuzzySearchOptions::new(1, 1, true),
        );

        let matches = matcher
            .search("spawnn", 10)
            .expect("lexical matcher succeeds");
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].matched_text, "spawn");
    }

    #[test]
    fn shared_prefix_len_handles_unicode_case_pairs() {
        assert_eq!(shared_prefix_len("Äpfel", "äPFEL"), 5);
    }

    #[test]
    fn lexical_matcher_respects_unicode_prefix_requirement() {
        let candidates = vec!["Äpfel".to_string(), "Banane".to_string()];
        fn string_value(candidate: &String) -> &str {
            candidate.as_str()
        }

        let matcher = LexicalMatcher::new(
            &candidates,
            string_value,
            FuzzySearchOptions::new(1, 1, true),
        );

        let matches = matcher
            .search("äpfelx", 10)
            .expect("unicode lexical matcher succeeds");
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].matched_text, "Äpfel");
    }

    #[test]
    fn lexical_matcher_clears_thread_local_buffers_between_searches() {
        let candidates = vec![
            "spawn".to_string(),
            "plan".to_string(),
            "storage".to_string(),
        ];
        fn string_value(candidate: &String) -> &str {
            candidate.as_str()
        }

        let matcher = LexicalMatcher::new(
            &candidates,
            string_value,
            FuzzySearchOptions::new(1, 1, true),
        );

        let first = matcher
            .search("spawnn", 10)
            .expect("first lexical matcher search succeeds");
        let second = matcher
            .search("plam", 10)
            .expect("second lexical matcher search succeeds");

        assert_eq!(first.len(), 1);
        assert_eq!(first[0].matched_text, "spawn");
        assert_eq!(second.len(), 1);
        assert_eq!(second[0].matched_text, "plan");
    }

    #[test]
    fn camel_case_symbol_profile_relaxes_prefix_length() {
        assert_eq!(FuzzySearchOptions::camel_case_symbol().prefix_length, 0);
    }
}
