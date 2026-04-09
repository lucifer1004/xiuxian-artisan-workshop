pub(crate) mod corpus;
pub(crate) mod issues;
pub(crate) mod maintenance;
pub(crate) mod phase;
pub(crate) mod reason;
pub(crate) mod snapshot;
pub(crate) mod telemetry;
#[cfg(test)]
#[path = "../../../tests/unit/search/status/mod.rs"]
mod tests;

pub use corpus::SearchCorpusStatus;
pub use issues::{
    SearchCorpusIssue, SearchCorpusIssueCode, SearchCorpusIssueFamily, SearchCorpusIssueSummary,
};
pub use maintenance::{SearchMaintenancePolicy, SearchMaintenanceStatus};
pub use phase::SearchPlanePhase;
pub use reason::{
    SearchCorpusStatusAction, SearchCorpusStatusReason, SearchCorpusStatusReasonCode,
    SearchCorpusStatusSeverity,
};
pub use snapshot::{SearchPlaneStatusSnapshot, SearchRepoReadPressure};
pub use telemetry::{SearchQueryTelemetry, SearchQueryTelemetrySource};
