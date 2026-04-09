//! Docs-facing query and response contracts for repository intelligence.

mod item;
mod queue;
mod rank;
mod search;
#[cfg(test)]
#[path = "../../../../../tests/unit/analyzers/query/docs/planner/mod.rs"]
mod tests;
mod workset;

pub use item::{DocsPlannerItemQuery, DocsPlannerItemResult};
pub use queue::{DocsPlannerQueueGroup, DocsPlannerQueueQuery, DocsPlannerQueueResult};
pub use rank::{
    DocsPlannerRankHit, DocsPlannerRankQuery, DocsPlannerRankReason, DocsPlannerRankReasonCode,
    DocsPlannerRankResult,
};
pub use search::{DocsPlannerSearchHit, DocsPlannerSearchQuery, DocsPlannerSearchResult};
pub use workset::{
    DocsPlannerWorksetBalance, DocsPlannerWorksetFamilyBalanceEntry, DocsPlannerWorksetFamilyGroup,
    DocsPlannerWorksetGapKindBalanceEntry, DocsPlannerWorksetGroup, DocsPlannerWorksetQuery,
    DocsPlannerWorksetQuotaHint, DocsPlannerWorksetResult, DocsPlannerWorksetStrategy,
    DocsPlannerWorksetStrategyCode, DocsPlannerWorksetStrategyReason,
    DocsPlannerWorksetStrategyReasonCode,
};
