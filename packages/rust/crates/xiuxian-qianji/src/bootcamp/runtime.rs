use crate::error::QianjiError;
use crate::runtime_config::resolve_process_project_root;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};
use xiuxian_wendao::link_graph::LinkGraphIndex;

pub(super) fn unix_timestamp_millis() -> Result<u128, QianjiError> {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .map_err(|error| {
            QianjiError::Execution(format!("system clock drifted before UNIX_EPOCH: {error}"))
        })
}

fn resolve_repo_root_path(explicit: Option<&Path>) -> PathBuf {
    if let Some(path) = explicit {
        return path.to_path_buf();
    }
    resolve_process_project_root().unwrap_or_else(std::env::temp_dir)
}

pub(super) fn build_link_graph_index(
    explicit_repo_root: Option<&Path>,
) -> Result<LinkGraphIndex, QianjiError> {
    let primary_root = resolve_repo_root_path(explicit_repo_root);
    match LinkGraphIndex::build(primary_root.as_path()) {
        Ok(index) => Ok(index),
        Err(primary_error) => {
            let fallback_root = std::env::temp_dir();
            LinkGraphIndex::build(fallback_root.as_path()).map_err(|fallback_error| {
                QianjiError::Topology(format!(
                    "failed to build LinkGraph index at `{}` ({primary_error}); \
fallback `{}` also failed ({fallback_error})",
                    primary_root.display(),
                    fallback_root.display()
                ))
            })
        }
    }
}
