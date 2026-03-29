use std::path::{Path, PathBuf};
use std::process::Child;

use crate::compatibility::link_graph::DEFAULT_JULIA_ANALYZER_PACKAGE_DIR;

pub(crate) struct ChildGuard {
    child: Child,
}

impl ChildGuard {
    pub(crate) fn new(child: Child) -> Self {
        Self { child }
    }

    pub(crate) fn kill(&mut self) {
        if let Some(_status) = self
            .child
            .try_wait()
            .unwrap_or_else(|error| panic!("poll WendaoArrow child: {error}"))
        {
            return;
        }
        self.child
            .kill()
            .unwrap_or_else(|error| panic!("kill WendaoArrow child: {error}"));
        let _ = self.child.wait();
    }
}

impl Drop for ChildGuard {
    fn drop(&mut self) {
        if let Ok(None) = self.child.try_wait() {
            let _ = self.child.kill();
            let _ = self.child.wait();
        }
    }
}

pub(crate) fn reserve_test_port() -> u16 {
    std::net::TcpListener::bind("127.0.0.1:0")
        .and_then(|listener| listener.local_addr())
        .map(|address| address.port())
        .unwrap_or_else(|error| panic!("reserve test port: {error}"))
}

pub(crate) fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../../../")
        .canonicalize()
        .unwrap_or_else(|error| panic!("resolve repo root: {error}"))
}

pub(crate) fn wendaoarrow_package_dir() -> PathBuf {
    repo_root()
        .join(".data/WendaoArrow")
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
