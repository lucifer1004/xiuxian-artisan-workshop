use std::fs;
use std::io::Error as IoError;
use std::net::{SocketAddr, TcpStream};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{Mutex, OnceLock};
use std::time::Duration;

use toml::Value;
use xiuxian_wendao_julia::integration_support::{
    JuliaExampleServiceGuard, spawn_wendaosearch_julia_parser_summary_service,
    spawn_wendaosearch_modelica_parser_summary_service,
};
use xiuxian_wendao_julia::{
    set_linked_julia_parser_summary_base_url_for_tests,
    set_linked_modelica_parser_summary_base_url_for_tests,
};

type TestResult = Result<(), Box<dyn std::error::Error>>;

const RUN_PROCESS_MANAGED_WENDAOSEARCH_TEST_ENV: &str = "RUN_PROCESS_MANAGED_WENDAOSEARCH_TEST";
const PROCESS_MANAGED_PARSER_SUMMARY_SERVICE_NAME: &str = "wendaosearch-parser-summary";

struct LinkedParserSummaryService {
    _guard: Mutex<JuliaExampleServiceGuard>,
}

pub fn ensure_linked_julia_parser_summary_service() -> TestResult {
    if process_managed_wendaosearch_test_enabled() {
        return ensure_process_managed_parser_summary_service();
    }
    static LINKED_JULIA_PARSER_SUMMARY_SERVICE: OnceLock<
        Result<LinkedParserSummaryService, String>,
    > = OnceLock::new();
    let service = LINKED_JULIA_PARSER_SUMMARY_SERVICE.get_or_init(|| {
        let (base_url, guard) = std::thread::spawn(|| {
            let runtime = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .map_err(|error| error.to_string())?;
            Ok::<(String, JuliaExampleServiceGuard), String>(
                runtime.block_on(spawn_wendaosearch_julia_parser_summary_service()),
            )
        })
        .join()
        .map_err(|_| "linked Julia parser-summary service thread panicked".to_string())??;
        set_linked_julia_parser_summary_base_url_for_tests(base_url.as_str())?;
        Ok(LinkedParserSummaryService {
            _guard: Mutex::new(guard),
        })
    });
    service
        .as_ref()
        .map(|_| ())
        .map_err(|message| Box::new(IoError::other(message.clone())) as Box<dyn std::error::Error>)
}

pub fn ensure_linked_modelica_parser_summary_service() -> TestResult {
    if process_managed_wendaosearch_test_enabled() {
        return ensure_process_managed_parser_summary_service();
    }
    static LINKED_MODELICA_PARSER_SUMMARY_SERVICE: OnceLock<
        Result<LinkedParserSummaryService, String>,
    > = OnceLock::new();
    let service = LINKED_MODELICA_PARSER_SUMMARY_SERVICE.get_or_init(|| {
        let (base_url, guard) = std::thread::spawn(|| {
            let runtime = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .map_err(|error| error.to_string())?;
            Ok::<(String, JuliaExampleServiceGuard), String>(
                runtime.block_on(spawn_wendaosearch_modelica_parser_summary_service()),
            )
        })
        .join()
        .map_err(|_| "linked Modelica parser-summary service thread panicked".to_string())??;
        set_linked_modelica_parser_summary_base_url_for_tests(base_url.as_str())?;
        Ok(LinkedParserSummaryService {
            _guard: Mutex::new(guard),
        })
    });
    service
        .as_ref()
        .map(|_| ())
        .map_err(|message| Box::new(IoError::other(message.clone())) as Box<dyn std::error::Error>)
}

fn process_managed_wendaosearch_test_enabled() -> bool {
    std::env::var_os(RUN_PROCESS_MANAGED_WENDAOSEARCH_TEST_ENV).is_some()
}

fn ensure_process_managed_parser_summary_service() -> TestResult {
    static PROCESS_MANAGED_PARSER_SUMMARY_SERVICE: OnceLock<Result<(), String>> = OnceLock::new();
    let service = PROCESS_MANAGED_PARSER_SUMMARY_SERVICE.get_or_init(|| {
        let base_url = process_managed_parser_summary_base_url()?;
        if !service_is_ready(base_url.as_str())? {
            let output = devenv_processes_command(["up", "-d", PROCESS_MANAGED_PARSER_SUMMARY_SERVICE_NAME])
                .output()
                .map_err(|error| {
                    format!(
                        "start process-managed `{PROCESS_MANAGED_PARSER_SUMMARY_SERVICE_NAME}` service: {error}"
                    )
                })?;
            if !output.status.success() {
                return Err(format!(
                    "start process-managed `{PROCESS_MANAGED_PARSER_SUMMARY_SERVICE_NAME}` service failed\nstdout:\n{}\nstderr:\n{}",
                    String::from_utf8_lossy(&output.stdout),
                    String::from_utf8_lossy(&output.stderr),
                ));
            }
            wait_for_service_ready(base_url.as_str(), 600)?;
        }
        set_linked_julia_parser_summary_base_url_for_tests(base_url.as_str())
            .map_err(|error| error.to_string())?;
        set_linked_modelica_parser_summary_base_url_for_tests(base_url.as_str())
            .map_err(|error| error.to_string())?;
        Ok(())
    });
    service
        .as_ref()
        .map(|_| ())
        .map_err(|message| Box::new(IoError::other(message.clone())) as Box<dyn std::error::Error>)
}

fn process_managed_parser_summary_base_url() -> Result<String, String> {
    let config_path = repo_root()
        .join(".data")
        .join("WendaoSearch.jl")
        .join("config")
        .join("live")
        .join("parser_summary.toml");
    let config_text = fs::read_to_string(&config_path)
        .map_err(|error| format!("read `{}`: {error}", config_path.display()))?;
    let config_value: Value = toml::from_str(&config_text)
        .map_err(|error| format!("parse `{}`: {error}", config_path.display()))?;
    let interface = config_value
        .get("interface")
        .and_then(Value::as_table)
        .ok_or_else(|| format!("`{}` is missing table `[interface]`", config_path.display()))?;
    let host = interface
        .get("host")
        .and_then(Value::as_str)
        .ok_or_else(|| format!("`{}` is missing string `[interface].host`", config_path.display()))?;
    let port = interface
        .get("port")
        .and_then(Value::as_integer)
        .ok_or_else(|| format!("`{}` is missing integer `[interface].port`", config_path.display()))?;
    Ok(format!("http://{host}:{port}"))
}

fn wait_for_service_ready(base_url: &str, attempts: usize) -> Result<(), String> {
    for _ in 0..attempts {
        if service_is_ready(base_url)? {
            return Ok(());
        }
        std::thread::sleep(Duration::from_millis(200));
    }
    Err(format!(
        "process-managed `{PROCESS_MANAGED_PARSER_SUMMARY_SERVICE_NAME}` did not become ready in time"
    ))
}

fn service_is_ready(base_url: &str) -> Result<bool, String> {
    let socket_addr = socket_addr_from_base_url(base_url)?;
    match TcpStream::connect_timeout(&socket_addr, Duration::from_secs(2)) {
        Ok(_) => Ok(true),
        Err(_) => Ok(false),
    }
}

fn socket_addr_from_base_url(base_url: &str) -> Result<SocketAddr, String> {
    let socket_addr = base_url
        .strip_prefix("http://")
        .or_else(|| base_url.strip_prefix("https://"))
        .unwrap_or(base_url);
    socket_addr
        .parse::<SocketAddr>()
        .map_err(|error| format!("parse socket address `{socket_addr}`: {error}"))
}

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(4)
        .expect("workspace root")
        .to_path_buf()
}

fn devenv_processes_command<const N: usize>(args: [&str; N]) -> Command {
    let mut command = Command::new("devenv");
    command
        .arg("processes")
        .args(args)
        .current_dir(repo_root())
        .env_remove("PC_CONFIG_FILES")
        .env_remove("PC_SOCKET_PATH");
    command
}
