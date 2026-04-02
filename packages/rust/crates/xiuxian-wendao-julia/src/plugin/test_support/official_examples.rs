use std::process::{Command, Stdio};
use std::time::Duration;

use tokio::net::TcpStream;
use tokio::time::sleep;

use super::common::{
    ChildGuard, repo_root, reserve_test_port, wendaoanalyzer_script, wendaoarrow_script,
};

pub(crate) fn spawn_real_wendaoarrow_service(port: u16) -> ChildGuard {
    let script = wendaoarrow_script("run_stream_scoring_flight_server.sh");
    spawn_example_script(script, port, "spawn real WendaoArrow service")
}

pub(crate) fn spawn_real_wendaoarrow_metadata_service(port: u16) -> ChildGuard {
    let script = wendaoarrow_script("run_stream_metadata_flight_server.sh");
    spawn_example_script(script, port, "spawn real WendaoArrow metadata service")
}

pub(crate) fn spawn_real_wendaoarrow_bad_response_service(port: u16) -> ChildGuard {
    let script = wendaoarrow_script("run_stream_scoring_bad_response_flight_server.sh");
    spawn_example_script(script, port, "spawn real WendaoArrow bad-response service")
}

pub(crate) fn spawn_real_wendaoanalyzer_linear_blend_service(port: u16) -> ChildGuard {
    let script = wendaoanalyzer_script("run_stream_linear_blend_server.sh");
    spawn_example_script(
        script,
        port,
        "spawn real WendaoAnalyzer linear blend service",
    )
}

fn spawn_example_script(script: std::path::PathBuf, port: u16, error_context: &str) -> ChildGuard {
    let child = Command::new("bash")
        .arg(script)
        .arg("--port")
        .arg(port.to_string())
        .current_dir(repo_root())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .unwrap_or_else(|error| panic!("{error_context}: {error}"));
    ChildGuard::new(child)
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

pub(crate) fn reserve_real_service_port() -> u16 {
    reserve_test_port()
}
