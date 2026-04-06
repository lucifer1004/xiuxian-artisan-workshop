use crate::backend::{BackendError, RepositoryHandle, checkout_detached_to_revision};
use crate::spec::RevisionSelector;

pub(crate) fn apply_revision(
    repository: &mut RepositoryHandle,
    revision: Option<&RevisionSelector>,
) -> Result<(), BackendError> {
    let Some(revision) = revision else {
        return Ok(());
    };

    let target = match revision {
        RevisionSelector::Branch(branch) => resolve_object_id(
            repository,
            &[
                format!("refs/remotes/origin/{branch}"),
                format!("refs/heads/{branch}"),
            ],
        )?,
        RevisionSelector::Tag(tag) => resolve_object_id(repository, &[format!("refs/tags/{tag}")])?,
        RevisionSelector::Commit(sha) => sha.clone(),
    };

    checkout_detached_to_revision(repository, target.as_str())
}

pub(crate) fn sync_checkout_head(
    repository: &mut RepositoryHandle,
    revision: Option<&RevisionSelector>,
) -> Result<(), BackendError> {
    if revision.is_some() {
        return apply_revision(repository, revision);
    }

    let target = resolve_object_id(
        repository,
        &[
            repository
                .find_reference("refs/remotes/origin/HEAD")
                .ok()
                .and_then(|reference| match reference.target() {
                    gix::refs::TargetRef::Symbolic(target) => Some(target.to_string()),
                    gix::refs::TargetRef::Object(_) => None,
                })
                .unwrap_or_else(|| "refs/remotes/origin/main".to_string()),
            repository
                .head_name()
                .ok()
                .flatten()
                .map(|name| name.shorten().to_string())
                .filter(|name| !name.is_empty() && name != "HEAD")
                .map_or_else(
                    || "refs/remotes/origin/main".to_string(),
                    |name| format!("refs/remotes/origin/{name}"),
                ),
            "refs/remotes/origin/main".to_string(),
            "refs/remotes/origin/master".to_string(),
            "refs/heads/main".to_string(),
            "refs/heads/master".to_string(),
            "refs/remotes/origin/HEAD".to_string(),
        ],
    )?;
    checkout_detached_to_revision(repository, target.as_str())
}

pub(crate) fn desired_managed_checkout_revision(
    mirror_repository: &RepositoryHandle,
    revision: Option<&RevisionSelector>,
) -> Option<String> {
    match revision {
        Some(RevisionSelector::Branch(branch)) => mirror_repository
            .find_reference(format!("refs/heads/{branch}").as_str())
            .ok()
            .and_then(|mut reference| reference.peel_to_id().ok().map(|oid| oid.to_string())),
        Some(RevisionSelector::Tag(tag)) => mirror_repository
            .find_reference(format!("refs/tags/{tag}").as_str())
            .ok()
            .and_then(|mut reference| reference.peel_to_id().ok().map(|oid| oid.to_string())),
        Some(RevisionSelector::Commit(sha)) => Some(sha.clone()),
        None => crate::metadata::resolve_head_revision(mirror_repository),
    }
}

fn resolve_object_id(
    repository: &RepositoryHandle,
    candidates: &[String],
) -> Result<String, BackendError> {
    for candidate in candidates {
        if let Ok(reference) = repository.rev_parse_single(candidate.as_str()) {
            return Ok(reference.to_string());
        }
    }

    Err(BackendError::new("git reference not found"))
}
