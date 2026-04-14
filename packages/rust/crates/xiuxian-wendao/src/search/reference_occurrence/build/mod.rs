mod extract;
mod orchestration;
mod plan;
mod types;
mod write;

#[cfg(test)]
#[path = "../../../../tests/unit/search/reference_occurrence/build/mod.rs"]
mod tests;

#[cfg(test)]
pub(crate) use orchestration::ensure_reference_occurrence_index_started;
pub(crate) use orchestration::ensure_reference_occurrence_index_started_with_scanned_files;
#[cfg(test)]
pub(crate) use orchestration::publish_reference_occurrences_from_projects;
pub(crate) use plan::plan_reference_occurrence_build_with_scanned_files;
#[cfg(test)]
pub(crate) use plan::{fingerprint_projects, plan_reference_occurrence_build};
#[cfg(test)]
pub(crate) use types::ReferenceOccurrenceBuildError;
pub(crate) use types::{ReferenceOccurrenceBuildPlan, ReferenceOccurrenceWriteResult};
pub(crate) use write::write_reference_occurrence_epoch;
