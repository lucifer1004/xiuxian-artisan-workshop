use std::path::PathBuf;
use std::process::{Command, Stdio};

use super::common::{
    JuliaExampleServiceGuard, repo_root, reserve_service_port, wait_for_service_ready,
    wait_for_service_ready_with_attempts, wendaoanalyzer_script, wendaoarrow_script,
    wendaosearch_package_dir, wendaosearch_script,
};
use crate::compatibility::link_graph::{
    LinkGraphJuliaAnalyzerLaunchManifest, LinkGraphJuliaDeploymentArtifact,
    LinkGraphJuliaRerankRuntimeConfig,
};

/// Spawns the official `WendaoArrow` stream-scoring Flight example service.
///
/// # Panics
///
/// Panics when the example script cannot be resolved or the service fails to
/// start.
pub async fn spawn_wendaoarrow_stream_scoring_service() -> (String, JuliaExampleServiceGuard) {
    spawn_script_service(
        wendaoarrow_script("run_stream_scoring_flight_server.sh"),
        "spawn real WendaoArrow service",
    )
    .await
}

/// Spawns the official `WendaoArrow` stream-metadata Flight example service.
///
/// # Panics
///
/// Panics when the example script cannot be resolved or the service fails to
/// start.
pub async fn spawn_wendaoarrow_stream_metadata_service() -> (String, JuliaExampleServiceGuard) {
    spawn_script_service(
        wendaoarrow_script("run_stream_metadata_flight_server.sh"),
        "spawn real WendaoArrow metadata service",
    )
    .await
}

/// Spawns the official `WendaoAnalyzer` linear-blend example service.
///
/// # Panics
///
/// Panics when the example script cannot be resolved or the service fails to
/// start.
pub async fn spawn_wendaoanalyzer_stream_linear_blend_service() -> (String, JuliaExampleServiceGuard)
{
    spawn_script_service(
        wendaoanalyzer_script("run_stream_linear_blend_server.sh"),
        "spawn real WendaoAnalyzer linear blend service",
    )
    .await
}

/// Spawns the official `WendaoSearch` structural-rerank example in `demo`
/// mode.
///
/// # Panics
///
/// Panics when the example script cannot be resolved or the service fails to
/// start.
pub async fn spawn_wendaosearch_demo_structural_rerank_service()
-> (String, JuliaExampleServiceGuard) {
    spawn_wendaosearch_service("structural_rerank", "demo").await
}

/// Spawns the official `WendaoSearch` structural-rerank example in
/// `solver_demo` mode.
///
/// # Panics
///
/// Panics when the example script cannot be resolved or the service fails to
/// start.
pub async fn spawn_wendaosearch_solver_demo_structural_rerank_service()
-> (String, JuliaExampleServiceGuard) {
    spawn_wendaosearch_service("structural_rerank", "solver_demo").await
}

/// Spawns the official same-port multi-route `WendaoSearch` example in `demo`
/// mode.
///
/// # Panics
///
/// Panics when the example script cannot be resolved or the service fails to
/// start.
pub async fn spawn_wendaosearch_demo_multi_route_service() -> (String, JuliaExampleServiceGuard) {
    spawn_wendaosearch_multi_route_service("demo").await
}

/// Spawns the official same-port multi-route `WendaoSearch` example in
/// `solver_demo` mode.
///
/// # Panics
///
/// Panics when the example script cannot be resolved or the service fails to
/// start.
pub async fn spawn_wendaosearch_solver_demo_multi_route_service()
-> (String, JuliaExampleServiceGuard) {
    spawn_wendaosearch_multi_route_service("solver_demo").await
}

/// Spawns the official `WendaoSearch` parser-summary service with the native
/// summary routes mounted on the shared Flight endpoint.
///
/// # Panics
///
/// Panics when the service script cannot be resolved or the service fails to
/// start.
pub async fn spawn_wendaosearch_julia_parser_summary_service() -> (String, JuliaExampleServiceGuard)
{
    spawn_wendaosearch_julia_parser_summary_service_with_attempts(1500).await
}

/// Spawns the official `WendaoSearch` parser-summary service with one explicit
/// readiness
/// attempt budget.
///
/// # Panics
///
/// Panics when the service script cannot be resolved or the service fails to
/// start.
pub async fn spawn_wendaosearch_julia_parser_summary_service_with_attempts(
    ready_attempts: usize,
) -> (String, JuliaExampleServiceGuard) {
    spawn_wendaosearch_parser_summary_service(ready_attempts).await
}

/// Spawns the official `WendaoSearch` parser-summary service for the Modelica
/// summary route.
///
/// # Panics
///
/// Panics when the service script cannot be resolved or the service fails to
/// start.
pub async fn spawn_wendaosearch_modelica_parser_summary_service()
-> (String, JuliaExampleServiceGuard) {
    spawn_wendaosearch_parser_summary_service(1500).await
}

/// Materializes a Julia deployment artifact from runtime-config values.
#[must_use]
pub fn wendaoanalyzer_deployment_artifact_from_runtime(
    runtime: &LinkGraphJuliaRerankRuntimeConfig,
) -> LinkGraphJuliaDeploymentArtifact {
    runtime.deployment_artifact()
}

/// Spawns a `WendaoAnalyzer` service from an explicit Julia launch manifest.
///
/// # Panics
///
/// Panics when the launcher path cannot be resolved, the child process cannot
/// be spawned, or the service never becomes ready.
pub async fn spawn_wendaoanalyzer_service_from_manifest(
    manifest: &LinkGraphJuliaAnalyzerLaunchManifest,
) -> (String, JuliaExampleServiceGuard) {
    let port = reserve_service_port();
    let base_url = format!("http://127.0.0.1:{port}");
    let script = repo_root().join(&manifest.launcher_path);
    let mut command = Command::new("bash");
    command.arg(script).arg("--port").arg(port.to_string());

    for argument in &manifest.args {
        command.arg(argument);
    }

    let child = command
        .current_dir(repo_root())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .unwrap_or_else(|error| panic!("spawn WendaoAnalyzer service: {error}"));
    let mut guard = JuliaExampleServiceGuard::new(child);

    wait_for_service_ready(base_url.as_str())
        .await
        .unwrap_or_else(|error| {
            guard.kill();
            panic!("wait for WendaoAnalyzer service readiness: {error}");
        });

    (base_url, guard)
}

/// Spawns a `WendaoAnalyzer` service from a rendered deployment artifact.
///
/// # Panics
///
/// Panics when the deployment artifact launcher cannot be spawned or the
/// service never becomes ready.
pub async fn spawn_wendaoanalyzer_service_from_artifact(
    artifact: &LinkGraphJuliaDeploymentArtifact,
) -> (String, JuliaExampleServiceGuard) {
    spawn_wendaoanalyzer_service_from_manifest(&artifact.launch).await
}

async fn spawn_script_service(
    script: PathBuf,
    error_context: &str,
) -> (String, JuliaExampleServiceGuard) {
    let port = reserve_service_port();
    let base_url = format!("http://127.0.0.1:{port}");
    let child = Command::new("bash")
        .arg(script)
        .arg("--port")
        .arg(port.to_string())
        .current_dir(repo_root())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .unwrap_or_else(|error| panic!("{error_context}: {error}"));
    let mut guard = JuliaExampleServiceGuard::new(child);

    wait_for_service_ready(base_url.as_str())
        .await
        .unwrap_or_else(|error| {
            guard.kill();
            panic!("wait for Julia official example service readiness: {error}");
        });

    (base_url, guard)
}

async fn spawn_wendaosearch_service(
    route_name: &str,
    mode: &str,
) -> (String, JuliaExampleServiceGuard) {
    spawn_wendaosearch_service_with_code_parser_routes(route_name, mode, &[], 600).await
}

async fn spawn_wendaosearch_parser_summary_service(
    ready_attempts: usize,
) -> (String, JuliaExampleServiceGuard) {
    let port = reserve_service_port();
    let base_url = format!("http://127.0.0.1:{port}");
    let script = wendaosearch_script("run_parser_summary_service.jl");
    let child = Command::new("direnv")
        .arg("exec")
        .arg(".")
        .arg("julia")
        .arg(format!(
            "--project={}",
            wendaosearch_package_dir().display()
        ))
        .arg(script)
        .arg("--host")
        .arg("127.0.0.1")
        .arg("--port")
        .arg(port.to_string())
        .current_dir(repo_root())
        .env("JULIA_LOAD_PATH", "@:@stdlib")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .unwrap_or_else(|error| panic!("spawn real WendaoSearch parser-summary service: {error}"));
    let mut guard = JuliaExampleServiceGuard::new(child);

    wait_for_service_ready_with_attempts(base_url.as_str(), ready_attempts)
        .await
        .unwrap_or_else(|error| {
            guard.kill();
            panic!("wait for WendaoSearch parser-summary service readiness: {error}");
        });

    (base_url, guard)
}

async fn spawn_wendaosearch_service_with_code_parser_routes(
    route_name: &str,
    mode: &str,
    code_parser_route_names: &[&str],
    ready_attempts: usize,
) -> (String, JuliaExampleServiceGuard) {
    let port = reserve_service_port();
    let base_url = format!("http://127.0.0.1:{port}");
    let script = wendaosearch_script("run_search_service.jl");
    let mut command = Command::new("direnv");
    command
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
        .stderr(Stdio::null());
    if !code_parser_route_names.is_empty() {
        command
            .arg("--code-parser-route-names")
            .arg(code_parser_route_names.join(","));
    }
    let child = command.spawn().unwrap_or_else(|error| {
        panic!("spawn real WendaoSearch `{route_name}` `{mode}` service: {error}")
    });
    let mut guard = JuliaExampleServiceGuard::new(child);

    wait_for_service_ready_with_attempts(base_url.as_str(), ready_attempts)
        .await
        .unwrap_or_else(|error| {
            guard.kill();
            panic!("wait for WendaoSearch service readiness: {error}");
        });

    (base_url, guard)
}

async fn spawn_wendaosearch_multi_route_service(mode: &str) -> (String, JuliaExampleServiceGuard) {
    let port = reserve_service_port();
    let base_url = format!("http://127.0.0.1:{port}");
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
    let mut guard = JuliaExampleServiceGuard::new(child);

    wait_for_service_ready_with_attempts(base_url.as_str(), 600)
        .await
        .unwrap_or_else(|error| {
            guard.kill();
            panic!("wait for WendaoSearch multi-route service readiness: {error}");
        });

    (base_url, guard)
}
