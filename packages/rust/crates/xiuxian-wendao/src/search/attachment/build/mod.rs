mod extract;
mod orchestration;
mod plan;
mod types;
mod write;

#[cfg(test)]
#[path = "../../../../tests/unit/search/attachment/build/mod.rs"]
mod tests;

pub(crate) use extract::attachment_kind_label;
#[cfg(test)]
pub(crate) use orchestration::ensure_attachment_index_started;
pub(crate) use orchestration::ensure_attachment_index_started_with_scanned_files;
#[cfg(test)]
pub(crate) use orchestration::publish_attachments_from_projects;
#[cfg(test)]
pub(crate) use plan::plan_attachment_build;
pub(crate) use plan::plan_attachment_build_with_scanned_files;
#[cfg(test)]
pub(crate) use types::AttachmentBuildError;
pub(crate) use types::{AttachmentBuildPlan, AttachmentWriteResult};
pub(crate) use write::write_attachment_epoch;
