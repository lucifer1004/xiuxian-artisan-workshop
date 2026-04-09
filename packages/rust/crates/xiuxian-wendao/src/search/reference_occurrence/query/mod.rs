mod search;
#[cfg(test)]
#[path = "../../../../tests/unit/search/reference_occurrence/query/mod.rs"]
mod tests;

pub(crate) use search::{ReferenceOccurrenceSearchError, search_reference_occurrences};
