use std::path::Path;

use super::error::{BackendError, error_message};
use super::retry::retry_git_open_operation;
use super::types::RepositoryHandle;

pub(crate) fn open_bare_with_retry(path: &Path) -> Result<RepositoryHandle, BackendError> {
    retry_git_open_operation(|| {
        let repository = gix::open(path.to_path_buf()).map_err(error_message)?;
        if repository.is_bare() {
            Ok(repository)
        } else {
            Err(BackendError::new(format!(
                "repository `{}` is not bare",
                path.display()
            )))
        }
    })
}

pub(crate) fn open_checkout_with_retry(path: &Path) -> Result<RepositoryHandle, BackendError> {
    retry_git_open_operation(|| {
        let repository = gix::open(path.to_path_buf()).map_err(error_message)?;
        if repository.is_bare() {
            Err(BackendError::new(format!(
                "repository `{}` is bare and cannot serve as checkout",
                path.display()
            )))
        } else {
            Ok(repository)
        }
    })
}
