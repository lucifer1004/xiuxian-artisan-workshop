//! Performance budget utilities for crate-level gate tests.
//!
//! This module offers a lightweight performance testing kernel that can be
//! reused by integration tests and gate entry points.

mod assert;
mod report;
mod run;
mod types;

pub use assert::assert_perf_budget;
pub use report::{PERF_REPORT_SCHEMA_VERSION, default_reports_root, report_output_path};
pub use run::{run_async_budget, run_sync_budget};
pub use types::{PerfBudget, PerfQuantiles, PerfReport, PerfRunConfig, PerfSummary};

#[cfg(test)]
#[path = "../../tests/unit/performance.rs"]
mod tests;
