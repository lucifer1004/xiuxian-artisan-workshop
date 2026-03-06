//! RRF Fusion - Reciprocal Rank Fusion algorithms for hybrid search.
//!
//! Layout: `kernels` (RRF term, distance→score), `types`, `rrf`, `weighted_rrf`, `adaptive_rrf`,
//! `match_util` (Aho-Corasick), `boost` (metadata / file-discovery).

mod adaptive_rrf;
mod boost;
mod kernels;
mod match_util;
mod rrf;
mod types;
mod weighted_rrf;

pub use adaptive_rrf::apply_adaptive_rrf;
pub use kernels::{distance_to_score, rrf_term, rrf_term_batch};
pub use match_util::{
    NameMatchResult, build_name_lower_arrow, build_name_token_automaton_with_phrase,
    count_name_token_matches_and_exact, lowercase_string_array,
};
pub use rrf::apply_rrf;
pub use types::HybridSearchResult;
pub use weighted_rrf::apply_weighted_rrf;
