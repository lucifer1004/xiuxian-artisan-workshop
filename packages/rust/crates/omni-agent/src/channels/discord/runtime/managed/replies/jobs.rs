use crate::channels::managed_runtime::replies as shared_replies;
use crate::jobs::{JobMetricsSnapshot, JobStatusSnapshot};

pub(in super::super) fn format_job_status(snapshot: &JobStatusSnapshot) -> String {
    shared_replies::format_job_status(snapshot)
}

pub(in super::super) fn format_job_metrics(metrics: &JobMetricsSnapshot) -> String {
    shared_replies::format_job_metrics(metrics)
}

pub(in super::super) fn format_job_not_found(job_id: &str) -> String {
    shared_replies::format_job_not_found(job_id)
}

pub(in super::super) fn format_job_status_json(snapshot: &JobStatusSnapshot) -> String {
    shared_replies::format_job_status_json(snapshot)
}

pub(in super::super) fn format_job_metrics_json(metrics: &JobMetricsSnapshot) -> String {
    shared_replies::format_job_metrics_json(metrics)
}

pub(in super::super) fn format_job_not_found_json(job_id: &str) -> String {
    shared_replies::format_job_not_found_json(job_id)
}
