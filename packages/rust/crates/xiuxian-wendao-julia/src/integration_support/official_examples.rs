use std::path::PathBuf;
use std::process::{Command, Stdio};

use super::common::{
    JuliaExampleServiceGuard, repo_root, reserve_service_port, wait_for_service_ready,
    wendaoanalyzer_script, wendaoarrow_script,
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
