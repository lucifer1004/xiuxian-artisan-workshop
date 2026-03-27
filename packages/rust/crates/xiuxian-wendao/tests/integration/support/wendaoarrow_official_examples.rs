use std::process::{Command, Stdio};

use super::wendaoarrow_common::{
    WendaoArrowServiceGuard, repo_root, reserve_test_port, wait_for_health, wendaoarrow_script,
};

pub(crate) async fn spawn_wendaoarrow_stream_scoring_service() -> (String, WendaoArrowServiceGuard)
{
    spawn_wendaoarrow_official_example(
        "run_stream_scoring_server.sh",
        "spawn WendaoArrow stream scoring service",
    )
    .await
}

pub(crate) async fn spawn_wendaoarrow_stream_metadata_service() -> (String, WendaoArrowServiceGuard)
{
    spawn_wendaoarrow_official_example(
        "run_stream_metadata_server.sh",
        "spawn WendaoArrow stream metadata service",
    )
    .await
}

async fn spawn_wendaoarrow_official_example(
    script_name: &str,
    error_context: &str,
) -> (String, WendaoArrowServiceGuard) {
    let port = reserve_test_port();
    let base_url = format!("http://127.0.0.1:{port}");
    let script = wendaoarrow_script(script_name);

    let child = Command::new("bash")
        .arg(script)
        .arg("--port")
        .arg(port.to_string())
        .current_dir(repo_root())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .unwrap_or_else(|error| panic!("{error_context}: {error}"));
    let guard = WendaoArrowServiceGuard::new(child);

    wait_for_health(base_url.as_str()).await;
    (base_url, guard)
}
