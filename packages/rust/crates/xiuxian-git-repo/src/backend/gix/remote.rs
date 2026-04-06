use std::fs::OpenOptions;
use std::path::{Path, PathBuf};

use gix::bstr::ByteSlice;
use gix::remote::Direction;

use super::constants::MIRROR_FETCH_REFSPEC;
use super::error::{BackendError, error_message};
use super::types::RepositoryHandle;

pub(crate) fn ensure_remote_url(
    repository: &mut RepositoryHandle,
    remote_name: &str,
    expected_url: &str,
) -> Result<bool, BackendError> {
    let expected_url = normalized_remote_url(repository, expected_url)?;
    let config_path = local_config_path(repository);
    let mut config = load_local_config(&config_path)?;

    match current_remote_url(repository, remote_name) {
        Some(current) if current == expected_url => Ok(false),
        Some(_) => {
            repository
                .find_remote(remote_name)
                .map_err(error_message)?
                .with_url_without_url_rewrite(expected_url.as_str())
                .map_err(error_message)?
                .save_to(&mut config)
                .map_err(error_message)?;
            write_local_config(&config, &config_path)?;
            repository.reload().map_err(error_message)?;
            Ok(true)
        }
        None => {
            let fetch_refspec = default_fetch_refspec(repository, remote_name);
            let mut remote = repository
                .remote_at_without_url_rewrite(expected_url.as_str())
                .map_err(error_message)?
                .with_refspecs(Some(fetch_refspec.as_str()), Direction::Fetch)
                .map_err(error_message)?;
            remote
                .save_as_to(remote_name, &mut config)
                .map_err(error_message)?;
            write_local_config(&config, &config_path)?;
            repository.reload().map_err(error_message)?;
            Ok(true)
        }
    }
}

fn normalized_remote_url(repository: &RepositoryHandle, url: &str) -> Result<String, BackendError> {
    repository
        .remote_at_without_url_rewrite(url)
        .map_err(error_message)?
        .url(Direction::Fetch)
        .map(display_remote_url)
        .ok_or_else(|| BackendError::new(format!("remote `{url}` did not expose a fetch url")))
}

fn local_config_path(repository: &RepositoryHandle) -> PathBuf {
    repository
        .config_snapshot()
        .plumbing()
        .meta()
        .path
        .clone()
        .unwrap_or_else(|| repository.git_dir().join("config"))
}

fn load_local_config(path: &Path) -> Result<gix::config::File<'static>, BackendError> {
    gix::config::File::from_path_no_includes(path.to_path_buf(), gix::config::Source::Local)
        .map_err(|error| {
            BackendError::new(format!(
                "failed to load local git config `{}`: {error}",
                path.display()
            ))
        })
}

fn write_local_config(
    config: &gix::config::File<'static>,
    path: &Path,
) -> Result<(), BackendError> {
    let mut file = OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(path)
        .map_err(|error| {
            BackendError::new(format!(
                "failed to open local git config `{}` for write: {error}",
                path.display()
            ))
        })?;
    config.write_to(&mut file).map_err(|error| {
        BackendError::new(format!(
            "failed to write local git config `{}`: {error}",
            path.display()
        ))
    })
}

fn current_remote_url(repository: &RepositoryHandle, remote_name: &str) -> Option<String> {
    repository
        .find_remote(remote_name)
        .ok()
        .and_then(|remote| remote.url(Direction::Fetch).map(display_remote_url))
}

fn display_remote_url(url: &gix::Url) -> String {
    if url.scheme == gix::url::Scheme::File {
        let path = gix::path::from_bstr(url.path.as_bstr()).into_owned();
        return std::fs::canonicalize(&path)
            .unwrap_or(path)
            .display()
            .to_string();
    }
    url.to_bstring().to_string()
}

fn default_fetch_refspec(repository: &RepositoryHandle, remote_name: &str) -> String {
    if repository.is_bare() {
        MIRROR_FETCH_REFSPEC.to_string()
    } else {
        format!("+refs/heads/*:refs/remotes/{remote_name}/*")
    }
}
