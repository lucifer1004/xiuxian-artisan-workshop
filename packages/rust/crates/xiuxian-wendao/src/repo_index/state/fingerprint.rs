use chrono::Utc;

use crate::analyzers::RegisteredRepository;

pub(super) fn fingerprint(repository: &RegisteredRepository) -> String {
    format!(
        "{}|{:?}|{:?}|{:?}|{:?}|{:?}",
        repository.id,
        repository.path,
        repository.url,
        repository.git_ref,
        repository.refresh,
        repository.repo_intelligence_plugin_ids()
    )
}

pub(super) fn fingerprint_id(fingerprint: &str) -> String {
    fingerprint
        .split('|')
        .next()
        .unwrap_or_default()
        .to_string()
}

pub(super) fn timestamp_now() -> String {
    Utc::now().to_rfc3339()
}
