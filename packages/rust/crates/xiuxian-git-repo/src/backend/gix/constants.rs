use std::time::Duration;

pub(crate) const MANAGED_REMOTE_RETRY_ATTEMPTS: usize = 3;
pub(crate) const MANAGED_GIT_OPEN_RETRY_ATTEMPTS: usize = 5;
pub(crate) const MANAGED_GIT_OPEN_RETRY_DELAY: Duration = Duration::from_millis(100);
pub(crate) const ORIGIN_REMOTE_NAME: &str = "origin";
pub(crate) const MIRROR_FETCH_REFSPEC: &str = "+refs/*:refs/*";
