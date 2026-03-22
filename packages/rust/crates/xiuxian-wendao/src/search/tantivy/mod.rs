//! Shared Tantivy-backed search primitives.

mod compare;
mod document;
mod fields;
mod fragments;
mod identifier;
mod index;
mod matcher;

pub use document::SearchDocument;
pub use fields::SearchDocumentFields;
pub use index::SearchDocumentIndex;
pub use matcher::{TantivyDocumentMatch, TantivyMatcher};

#[cfg(test)]
mod tests;
