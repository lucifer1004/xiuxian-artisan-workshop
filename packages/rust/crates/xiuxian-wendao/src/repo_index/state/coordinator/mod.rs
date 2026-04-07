#[cfg(feature = "performance")]
mod diagnostics;
mod handle;
mod hydration;
mod lifecycle;
mod queue;
mod runtime;
mod status;
mod types;

#[cfg(test)]
pub(crate) use runtime::PreparedIncrementalAnalysis;
pub(crate) use types::RepoIndexCoordinator;
