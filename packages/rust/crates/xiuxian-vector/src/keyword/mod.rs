//! keyword.rs - keyword fusion helpers for `xiuxian-vector`.
//!
//! This module keeps the ranking/fusion helpers used by hybrid search. The
//! runtime keyword retrieval path now converges on Lance FTS only.

pub mod entity_aware;
pub mod fusion;

pub use entity_aware::{
    ENTITY_CONFIDENCE_THRESHOLD, ENTITY_WEIGHT, EntityAwareSearchResult, EntityMatch,
    EntityMatchType, MAX_ENTITY_MATCHES, apply_entity_boost, apply_triple_rrf,
};
pub use fusion::{
    HybridSearchResult, apply_adaptive_rrf, apply_rrf, apply_weighted_rrf, distance_to_score,
    rrf_term, rrf_term_batch,
};
use serde::{Deserialize, Serialize};

/// Configurable keyword backend for hybrid retrieval.
///
/// Only Lance FTS remains supported; the enum stays to preserve the explicit
/// runtime configuration surface.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum KeywordSearchBackend {
    /// Lance native inverted index / FTS path.
    #[default]
    LanceFts,
}

/// Default RRF k parameter for high precision (Code Search)
/// Based on `MariaDB` Engineering (2025): k=10 is optimal for precision-critical scenarios
pub const RRF_K: f32 = 10.0;

/// Semantic weight for hybrid search (vector contribution)
pub const SEMANTIC_WEIGHT: f32 = 1.0;

/// Keyword weight for hybrid search (BM25 contribution)
/// Keywords are precise anchors for code/tools, so we weight them higher
pub const KEYWORD_WEIGHT: f32 = 1.5;

/// Boost for exact token match in tool name (e.g., "commit" in "git.commit")
/// This is in RRF score space (~0.1 per rank), so boost should be small
pub const NAME_TOKEN_BOOST: f32 = 0.3;
/// Boost for exact phrase match in tool name
pub const EXACT_PHRASE_BOOST: f32 = 0.5;
