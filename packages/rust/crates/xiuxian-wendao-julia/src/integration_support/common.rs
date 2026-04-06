use std::path::{Path, PathBuf};
use std::process::Child;
use std::time::Duration;

use crate::compatibility::link_graph::{
    DEFAULT_JULIA_ANALYZER_PACKAGE_DIR, DEFAULT_JULIA_ARROW_PACKAGE_DIR,
};
use tokio::net::TcpStream;
use tokio::time::sleep;

/// Guard for a spawned Julia integration-support service process.
pub struct JuliaExampleServiceGuard {
    child: Child,
}

impl JuliaExampleServiceGuard {
    pub(crate) fn new(child: Child) -> Self {
        Self { child }
    }

    /// Terminates the spawned service if it is still running.
    ///
    /// # Panics
    ///
    /// Panics when polling or terminating the child process fails.
    pub fn kill(&mut self) {
        if let Some(_status) = self
            .child
            .try_wait()
            .unwrap_or_else(|error| panic!("poll Julia example child: {error}"))
        {
            return;
        }
        self.child
            .kill()
            .unwrap_or_else(|error| panic!("kill Julia example child: {error}"));
        let _ = self.child.wait();
    }
}

impl Drop for JuliaExampleServiceGuard {
    fn drop(&mut self) {
        if let Ok(None) = self.child.try_wait() {
            let _ = self.child.kill();
            let _ = self.child.wait();
        }
    }
}

pub(crate) fn reserve_service_port() -> u16 {
    std::net::TcpListener::bind("127.0.0.1:0")
        .and_then(|listener| listener.local_addr())
        .map_or_else(
            |error| panic!("reserve Julia example service port: {error}"),
            |address| address.port(),
        )
}

pub(crate) fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../../../")
        .canonicalize()
        .unwrap_or_else(|error| panic!("resolve repo root: {error}"))
}

pub(crate) fn wendaoarrow_package_dir() -> PathBuf {
    repo_root()
        .join(DEFAULT_JULIA_ARROW_PACKAGE_DIR)
        .canonicalize()
        .unwrap_or_else(|error| panic!("resolve WendaoArrow package dir: {error}"))
}

pub(crate) fn wendaoarrow_script(name: &str) -> PathBuf {
    wendaoarrow_package_dir()
        .join("scripts")
        .join(name)
        .canonicalize()
        .unwrap_or_else(|error| panic!("resolve WendaoArrow script `{name}`: {error}"))
}

pub(crate) fn wendaoanalyzer_package_dir() -> PathBuf {
    repo_root()
        .join(DEFAULT_JULIA_ANALYZER_PACKAGE_DIR)
        .canonicalize()
        .unwrap_or_else(|error| panic!("resolve WendaoAnalyzer package dir: {error}"))
}

pub(crate) fn wendaoanalyzer_script(name: &str) -> PathBuf {
    wendaoanalyzer_package_dir()
        .join("scripts")
        .join(name)
        .canonicalize()
        .unwrap_or_else(|error| panic!("resolve WendaoAnalyzer script `{name}`: {error}"))
}

pub(crate) fn wendaosearch_package_dir() -> PathBuf {
    repo_root()
        .join(".data/WendaoSearch.jl")
        .canonicalize()
        .unwrap_or_else(|error| panic!("resolve WendaoSearch package dir: {error}"))
}

pub(crate) fn wendaosearch_script(name: &str) -> PathBuf {
    wendaosearch_package_dir()
        .join("scripts")
        .join(name)
        .canonicalize()
        .unwrap_or_else(|error| panic!("resolve WendaoSearch script `{name}`: {error}"))
}

pub(crate) async fn wait_for_service_ready(base_url: &str) -> Result<(), String> {
    wait_for_service_ready_with_attempts(base_url, 150).await
}

pub(crate) async fn wait_for_service_ready_with_attempts(
    base_url: &str,
    attempts: usize,
) -> Result<(), String> {
    let socket_addr = base_url
        .strip_prefix("http://")
        .or_else(|| base_url.strip_prefix("https://"))
        .unwrap_or(base_url)
        .to_string();

    for _ in 0..attempts {
        if TcpStream::connect(&socket_addr).await.is_ok() {
            return Ok(());
        }
        sleep(Duration::from_millis(200)).await;
    }

    Err("real Julia Flight service did not become ready in time".to_string())
}
