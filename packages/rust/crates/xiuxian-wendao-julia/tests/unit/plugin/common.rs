use std::fmt::Display;
use std::io::Error as IoError;
use std::path::{Path, PathBuf};
use std::process::Child;
use std::sync::{Mutex, OnceLock};

use crate::compatibility::link_graph::{
    DEFAULT_JULIA_ANALYZER_PACKAGE_DIR, DEFAULT_JULIA_ARROW_PACKAGE_DIR,
};
use crate::integration_support::{
    JuliaExampleServiceGuard, spawn_wendaosearch_demo_julia_parser_summary_service,
    spawn_wendaosearch_demo_modelica_parser_summary_service,
};
use crate::{
    set_linked_julia_parser_summary_base_url_for_tests,
    set_linked_modelica_parser_summary_base_url_for_tests,
};

pub(crate) struct ChildGuard {
    child: Child,
}

struct LinkedJuliaParserSummaryService {
    _guard: Mutex<JuliaExampleServiceGuard>,
}

struct LinkedModelicaParserSummaryService {
    _guard: Mutex<JuliaExampleServiceGuard>,
}

static LINKED_JULIA_PARSER_SUMMARY_SERVICE: OnceLock<
    Result<LinkedJuliaParserSummaryService, String>,
> = OnceLock::new();
static LINKED_MODELICA_PARSER_SUMMARY_SERVICE: OnceLock<
    Result<LinkedModelicaParserSummaryService, String>,
> = OnceLock::new();

pub(crate) trait ResultTestExt<T, E> {
    fn or_panic(self, context: &str) -> T;
    fn err_or_panic(self, context: &str) -> E;
}

impl<T, E> ResultTestExt<T, E> for Result<T, E>
where
    E: Display,
{
    fn or_panic(self, context: &str) -> T {
        self.unwrap_or_else(|error| panic!("{context}: {error}"))
    }

    fn err_or_panic(self, context: &str) -> E {
        let Err(error) = self else {
            panic!("{context}");
        };
        error
    }
}

pub(crate) trait OptionTestExt<T> {
    fn or_panic(self, context: &str) -> T;
}

impl<T> OptionTestExt<T> for Option<T> {
    fn or_panic(self, context: &str) -> T {
        let Some(value) = self else {
            panic!("{context}");
        };
        value
    }
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
        .map_or_else(
            |error| panic!("reserve test port: {error}"),
            |address| address.port(),
        )
}

pub(crate) fn assert_f64_eq(actual: f64, expected: f64) {
    let delta = (actual - expected).abs();
    assert!(
        delta <= 1.0e-12,
        "expected `{expected}` but got `{actual}` (delta: {delta})"
    );
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

pub(crate) fn wendaoanalyzer_package_dir() -> PathBuf {
    repo_root()
        .join(DEFAULT_JULIA_ANALYZER_PACKAGE_DIR)
        .canonicalize()
        .unwrap_or_else(|error| panic!("resolve WendaoAnalyzer package dir: {error}"))
}

pub(crate) fn wendaosearch_package_dir() -> PathBuf {
    repo_root()
        .join(".data/WendaoSearch.jl")
        .canonicalize()
        .unwrap_or_else(|error| panic!("resolve WendaoSearch package dir: {error}"))
}

pub(crate) fn wendaosearch_config(name: &str) -> PathBuf {
    wendaosearch_package_dir()
        .join("config")
        .join("live")
        .join(name)
        .canonicalize()
        .unwrap_or_else(|error| panic!("resolve WendaoSearch config `{name}`: {error}"))
}

pub(crate) fn wendaosearch_script(name: &str) -> PathBuf {
    wendaosearch_package_dir()
        .join("scripts")
        .join(name)
        .canonicalize()
        .unwrap_or_else(|error| panic!("resolve WendaoSearch script `{name}`: {error}"))
}

pub(crate) fn ensure_linked_julia_parser_summary_service() -> Result<(), Box<dyn std::error::Error>>
{
    let service = LINKED_JULIA_PARSER_SUMMARY_SERVICE.get_or_init(|| {
        let (base_url, guard) = std::thread::spawn(|| {
            let runtime = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .map_err(|error| error.to_string())?;
            Ok::<(String, JuliaExampleServiceGuard), String>(
                runtime.block_on(spawn_wendaosearch_demo_julia_parser_summary_service()),
            )
        })
        .join()
        .map_err(|_| "linked Julia parser-summary service thread panicked".to_string())??;
        set_linked_julia_parser_summary_base_url_for_tests(base_url.as_str())?;
        Ok::<LinkedJuliaParserSummaryService, String>(LinkedJuliaParserSummaryService {
            _guard: Mutex::new(guard),
        })
    });
    service
        .as_ref()
        .map(|_| ())
        .map_err(|message| Box::new(IoError::other(message.clone())) as Box<dyn std::error::Error>)
}

pub(crate) fn ensure_linked_modelica_parser_summary_service()
-> Result<(), Box<dyn std::error::Error>> {
    let service = LINKED_MODELICA_PARSER_SUMMARY_SERVICE.get_or_init(|| {
        let (base_url, guard) = std::thread::spawn(|| {
            let runtime = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .map_err(|error| error.to_string())?;
            Ok::<(String, JuliaExampleServiceGuard), String>(
                runtime.block_on(spawn_wendaosearch_demo_modelica_parser_summary_service()),
            )
        })
        .join()
        .map_err(|_| "linked Modelica parser-summary service thread panicked".to_string())??;
        set_linked_modelica_parser_summary_base_url_for_tests(base_url.as_str())?;
        Ok::<LinkedModelicaParserSummaryService, String>(LinkedModelicaParserSummaryService {
            _guard: Mutex::new(guard),
        })
    });
    service
        .as_ref()
        .map(|_| ())
        .map_err(|message| Box::new(IoError::other(message.clone())) as Box<dyn std::error::Error>)
}
