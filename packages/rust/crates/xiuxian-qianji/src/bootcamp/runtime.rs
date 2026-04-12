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

fn build_link_graph_index_for_root_with_builders<C, P>(
    root: &Path,
    cache_build: &C,
    plain_build: &P,
) -> Result<LinkGraphIndex, String>
where
    C: Fn(&Path) -> Result<LinkGraphIndex, String>,
    P: Fn(&Path) -> Result<LinkGraphIndex, String>,
{
    match cache_build(root) {
        Ok(index) => Ok(index),
        Err(cache_error) => plain_build(root).map_err(|plain_error| {
            format!("cache bootstrap failed ({cache_error}); build fallback failed ({plain_error})")
        }),
    }
}

fn build_link_graph_index_with_builders<C, P>(
    primary_root: &Path,
    fallback_root: &Path,
    cache_build: C,
    plain_build: P,
) -> Result<LinkGraphIndex, QianjiError>
where
    C: Fn(&Path) -> Result<LinkGraphIndex, String>,
    P: Fn(&Path) -> Result<LinkGraphIndex, String>,
{
    match build_link_graph_index_for_root_with_builders(primary_root, &cache_build, &plain_build) {
        Ok(index) => Ok(index),
        Err(primary_error) => {
            build_link_graph_index_for_root_with_builders(fallback_root, &cache_build, &plain_build)
                .map_err(|fallback_error| {
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

pub(super) fn build_link_graph_index(
    explicit_repo_root: Option<&Path>,
) -> Result<LinkGraphIndex, QianjiError> {
    let primary_root = resolve_repo_root_path(explicit_repo_root);
    let fallback_root = std::env::temp_dir();
    build_link_graph_index_with_builders(
        primary_root.as_path(),
        fallback_root.as_path(),
        |root| LinkGraphIndex::build_with_cache(root, &[], &[]),
        LinkGraphIndex::build,
    )
}

#[cfg(test)]
mod tests {
    use super::build_link_graph_index_with_builders;
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::sync::{Arc, Mutex};
    use tempfile::tempdir;
    use xiuxian_wendao::link_graph::LinkGraphIndex;

    #[test]
    fn build_link_graph_index_falls_back_to_plain_build_when_cache_bootstrap_fails() {
        let root = tempdir().unwrap_or_else(|error| panic!("tempdir should succeed: {error}"));
        let index = build_link_graph_index_with_builders(
            root.path(),
            root.path(),
            |_| Err("cache unavailable".to_string()),
            LinkGraphIndex::build,
        )
        .unwrap_or_else(|error| panic!("plain build fallback should succeed: {error}"));
        assert_eq!(
            fs::canonicalize(index.root())
                .unwrap_or_else(|error| panic!("index root should canonicalize: {error}")),
            fs::canonicalize(root.path())
                .unwrap_or_else(|error| panic!("temp root should canonicalize: {error}"))
        );
    }

    #[test]
    fn build_link_graph_index_tries_fallback_root_after_primary_root_failure() {
        let fallback_root =
            tempdir().unwrap_or_else(|error| panic!("tempdir should succeed: {error}"));
        let fallback_root_path = fallback_root.path().to_path_buf();
        let primary_root = fallback_root.path().join("missing-primary-root");
        let seen_roots = Arc::new(Mutex::new(Vec::<PathBuf>::new()));

        let index = build_link_graph_index_with_builders(
            primary_root.as_path(),
            fallback_root_path.as_path(),
            |_| Err("cache unavailable".to_string()),
            {
                let seen_roots = Arc::clone(&seen_roots);
                let fallback_root_path = fallback_root_path.clone();
                move |root: &Path| {
                    seen_roots
                        .lock()
                        .unwrap_or_else(std::sync::PoisonError::into_inner)
                        .push(root.to_path_buf());
                    if root == fallback_root_path.as_path() {
                        LinkGraphIndex::build(root)
                    } else {
                        Err("forced primary failure".to_string())
                    }
                }
            },
        )
        .unwrap_or_else(|error| panic!("fallback root build should succeed: {error}"));

        let roots = seen_roots
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clone();
        assert_eq!(
            roots,
            vec![primary_root.clone(), fallback_root_path.clone()]
        );
        assert_eq!(
            fs::canonicalize(index.root())
                .unwrap_or_else(|error| panic!("index root should canonicalize: {error}")),
            fs::canonicalize(fallback_root_path.as_path())
                .unwrap_or_else(|error| panic!("fallback root should canonicalize: {error}"))
        );
    }
}
