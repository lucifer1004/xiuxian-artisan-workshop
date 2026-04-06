use gix::bstr::ByteSlice;
use gix::remote::Direction;

use crate::spec::RevisionSelector;

use super::error::{BackendError, error_message};
use super::retry::retry_remote_operation;
use super::types::RepositoryHandle;

pub(crate) fn probe_remote_target_revision_with_retry(
    repository: &RepositoryHandle,
    revision: Option<&RevisionSelector>,
) -> Result<Option<String>, BackendError> {
    retry_remote_operation(|| probe_remote_target_revision_once(repository, revision))
}

fn probe_remote_target_revision_once(
    repository: &RepositoryHandle,
    revision: Option<&RevisionSelector>,
) -> Result<Option<String>, BackendError> {
    if let Some(RevisionSelector::Commit(sha)) = revision {
        return Ok(Some(sha.clone()));
    }

    let remote = repository.find_remote("origin").map_err(error_message)?;
    let connection = remote.connect(Direction::Fetch).map_err(error_message)?;
    let (ref_map, _handshake) = connection
        .ref_map(gix::progress::Discard, remote_probe_options(revision)?)
        .map_err(error_message)?;

    Ok(match revision {
        Some(RevisionSelector::Branch(branch)) => remote_ref_target_revision(
            &ref_map.remote_refs,
            format!("refs/heads/{branch}").as_str(),
        ),
        Some(RevisionSelector::Tag(tag)) => {
            remote_ref_target_revision(&ref_map.remote_refs, format!("refs/tags/{tag}").as_str())
        }
        None => default_remote_head_revision(&ref_map.remote_refs),
        Some(RevisionSelector::Commit(_)) => unreachable!("commit revisions return early"),
    })
}

pub(super) fn remote_probe_options(
    revision: Option<&RevisionSelector>,
) -> Result<gix::remote::ref_map::Options, BackendError> {
    let probe_refspec = match revision {
        Some(RevisionSelector::Branch(branch)) => Some(format!("refs/heads/{branch}")),
        Some(RevisionSelector::Tag(tag)) => Some(format!("refs/tags/{tag}")),
        Some(RevisionSelector::Commit(_)) => None,
        None => Some("HEAD".to_string()),
    };

    let mut options = gix::remote::ref_map::Options {
        prefix_from_spec_as_filter_on_remote: false,
        ..Default::default()
    };
    if let Some(probe_refspec) = probe_refspec {
        options.extra_refspecs.push(
            gix::refspec::parse(
                probe_refspec.as_str().into(),
                gix::refspec::parse::Operation::Fetch,
            )
            .map_err(error_message)?
            .to_owned(),
        );
    }
    Ok(options)
}

pub(super) fn default_remote_head_revision(
    remote_refs: &[gix::protocol::handshake::Ref],
) -> Option<String> {
    remote_ref_target_revision(remote_refs, "HEAD")
}

pub(super) fn remote_ref_target_revision(
    remote_refs: &[gix::protocol::handshake::Ref],
    full_ref_name: &str,
) -> Option<String> {
    remote_refs
        .iter()
        .find(|reference| remote_ref_name(reference) == full_ref_name.as_bytes().as_bstr())
        .and_then(remote_ref_revision)
}

fn remote_ref_name(reference: &gix::protocol::handshake::Ref) -> &gix::bstr::BStr {
    match reference {
        gix::protocol::handshake::Ref::Peeled { full_ref_name, .. }
        | gix::protocol::handshake::Ref::Direct { full_ref_name, .. }
        | gix::protocol::handshake::Ref::Symbolic { full_ref_name, .. }
        | gix::protocol::handshake::Ref::Unborn { full_ref_name, .. } => full_ref_name.as_bstr(),
    }
}

fn remote_ref_revision(reference: &gix::protocol::handshake::Ref) -> Option<String> {
    match reference {
        gix::protocol::handshake::Ref::Peeled { object, .. }
        | gix::protocol::handshake::Ref::Direct { object, .. }
        | gix::protocol::handshake::Ref::Symbolic { object, .. } => Some(object.to_string()),
        gix::protocol::handshake::Ref::Unborn { .. } => None,
    }
}
