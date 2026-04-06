use std::process::{Command, Stdio};
use std::time::Duration;

use tokio::net::TcpStream;
use tokio::time::sleep;

use super::common::{
    ChildGuard, repo_root, reserve_test_port, wendaoanalyzer_script, wendaoarrow_script,
    wendaosearch_package_dir, wendaosearch_script,
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

pub(crate) fn spawn_real_wendaosearch_demo_structural_rerank_service(port: u16) -> ChildGuard {
    spawn_real_wendaosearch_service("structural_rerank", "demo", port)
}

pub(crate) fn spawn_real_wendaosearch_demo_capability_manifest_service(port: u16) -> ChildGuard {
    spawn_real_wendaosearch_service("capability_manifest", "demo", port)
}

pub(crate) fn spawn_real_wendaosearch_solver_demo_structural_rerank_service(
    port: u16,
) -> ChildGuard {
    spawn_real_wendaosearch_service("structural_rerank", "solver_demo", port)
}

pub(crate) fn spawn_real_wendaosearch_solver_demo_constraint_filter_service(
    port: u16,
) -> ChildGuard {
    spawn_real_wendaosearch_service("constraint_filter", "solver_demo", port)
}

pub(crate) fn spawn_real_wendaosearch_demo_multi_route_service(port: u16) -> ChildGuard {
    spawn_real_wendaosearch_multi_route_service("demo", port)
}

pub(crate) fn spawn_real_wendaosearch_solver_demo_multi_route_service(port: u16) -> ChildGuard {
    spawn_real_wendaosearch_multi_route_service("solver_demo", port)
}

fn spawn_real_wendaosearch_multi_route_service(mode: &str, port: u16) -> ChildGuard {
    let script = wendaosearch_script("run_search_service.jl");
    let child = Command::new("direnv")
        .arg("exec")
        .arg(".")
        .arg("julia")
        .arg(format!(
            "--project={}",
            wendaosearch_package_dir().display()
        ))
        .arg(script)
        .arg("--route-names")
        .arg("capability_manifest,structural_rerank,constraint_filter")
        .arg("--mode")
        .arg(mode)
        .arg("--host")
        .arg("127.0.0.1")
        .arg("--port")
        .arg(port.to_string())
        .current_dir(repo_root())
        .env("JULIA_LOAD_PATH", "@:@stdlib")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .unwrap_or_else(|error| {
            panic!("spawn real WendaoSearch multi-route `{mode}` service: {error}")
        });
    ChildGuard::new(child)
}

fn spawn_real_wendaosearch_service(route_name: &str, mode: &str, port: u16) -> ChildGuard {
    let script = wendaosearch_script("run_search_service.jl");
    let child = Command::new("direnv")
        .arg("exec")
        .arg(".")
        .arg("julia")
        .arg(format!(
            "--project={}",
            wendaosearch_package_dir().display()
        ))
        .arg(script)
        .arg("--route-name")
        .arg(route_name)
        .arg("--mode")
        .arg(mode)
        .arg("--host")
        .arg("127.0.0.1")
        .arg("--port")
        .arg(port.to_string())
        .current_dir(repo_root())
        .env("JULIA_LOAD_PATH", "@:@stdlib")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .unwrap_or_else(|error| {
            panic!("spawn real WendaoSearch `{route_name}` `{mode}` service: {error}")
        });
    ChildGuard::new(child)
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
