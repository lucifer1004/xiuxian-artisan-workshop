from __future__ import annotations

import os
import socket
import subprocess
import time
from pathlib import Path

import pytest


def _package_root() -> Path:
    return Path(__file__).resolve().parents[1]


def _project_root() -> Path:
    project_root = os.environ.get("PRJ_ROOT")
    if not project_root:
        pytest.skip("set PRJ_ROOT before running analyzer example integration tests")
    return Path(project_root)


def _wendao_search_flight_server_binary() -> Path:
    return (
        _project_root() / ".cache" / "pyflight-f56-target" / "debug" / "wendao_search_flight_server"
    )


def _wendao_search_seed_binary() -> Path:
    return (
        _project_root() / ".cache" / "pyflight-f56-target" / "debug" / "wendao_search_seed_sample"
    )


def _wendao_runtime_flight_server_binary() -> Path:
    return (
        _project_root()
        / ".cache"
        / "pyflight-rust-contract-target"
        / "debug"
        / "wendao_flight_server"
    )


def _run_rust_search_plane_seed_binary(project_root: Path, *, repo_id: str = "alpha/repo") -> None:
    binary = Path(os.environ.get("WENDAO_SEARCH_SEED_BINARY", str(_wendao_search_seed_binary())))
    if not binary.exists():
        pytest.skip(f"build {binary} before running analyzer example integration tests")

    result = subprocess.run(
        [str(binary), repo_id, str(project_root)],
        cwd=_project_root(),
        check=False,
        capture_output=True,
        text=True,
    )
    if result.returncode != 0:
        raise AssertionError(
            "Wendao search-plane seed binary failed:\n"
            f"stdout:\n{result.stdout}\n"
            f"stderr:\n{result.stderr}"
        )


def _spawn_wendao_search_flight_server(
    host: str, port: int, project_root: Path
) -> subprocess.Popen[str]:
    binary = Path(
        os.environ.get("WENDAO_SEARCH_SERVER_BINARY", str(_wendao_search_flight_server_binary()))
    )
    if not binary.exists():
        pytest.skip(f"build {binary} before running analyzer example integration tests")

    process = subprocess.Popen(
        [
            str(binary),
            f"{host}:{port}",
            "--schema-version=v2",
            "alpha/repo",
            str(project_root),
            "3",
        ],
        cwd=project_root,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
        env={**os.environ, "PRJ_ROOT": str(project_root)},
    )
    deadline = time.time() + 120
    while time.time() < deadline:
        line = process.stdout.readline() if process.stdout is not None else ""
        if line.startswith("READY http://"):
            time.sleep(1.0)
            return process
        if process.poll() is not None:
            stderr = process.stderr.read() if process.stderr is not None else ""
            raise AssertionError(f"Wendao search Flight server exited before readiness:\n{stderr}")

    raise AssertionError("timed out waiting for Wendao search Flight server readiness")


def _spawn_wendao_runtime_flight_server(host: str, port: int) -> subprocess.Popen[str]:
    binary = Path(
        os.environ.get("WENDAO_RUNTIME_SERVER_BINARY", str(_wendao_runtime_flight_server_binary()))
    )
    if not binary.exists():
        pytest.skip(f"build {binary} before running analyzer example integration tests")

    process = subprocess.Popen(
        [
            str(binary),
            f"{host}:{port}",
            "--schema-version=v2",
            "--rerank-dimension=3",
        ],
        cwd=_project_root(),
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
        env=os.environ.copy(),
    )
    deadline = time.time() + 120
    while time.time() < deadline:
        line = process.stdout.readline() if process.stdout is not None else ""
        if line.startswith("READY http://"):
            time.sleep(1.0)
            return process
        if process.poll() is not None:
            stderr = process.stderr.read() if process.stderr is not None else ""
            raise AssertionError(f"Wendao runtime Flight server exited before readiness:\n{stderr}")

    raise AssertionError("timed out waiting for Wendao runtime Flight server readiness")


def _terminate_process(process: subprocess.Popen[str]) -> None:
    process.terminate()
    try:
        process.wait(timeout=10)
    except subprocess.TimeoutExpired:
        process.kill()
        process.wait(timeout=10)


def _run_example_via_uv(*args: str) -> subprocess.CompletedProcess[str]:
    return subprocess.run(
        ["uv", "run", "python", *args],
        cwd=_package_root(),
        check=True,
        capture_output=True,
        text=True,
    )


def test_shipped_example_set_matches_current_beta_freeze() -> None:
    example_names = {
        path.name for path in (_package_root() / "examples").glob("*.py") if path.is_file()
    }

    assert example_names == {
        "custom_repo_analyzer_workflow.py",
        "host_backed_repo_search_beta_smoke.py",
        "host_backed_rerank_beta_smoke.py",
        "local_rerank_workflow.py",
        "repo_search_workflow.py",
        "rerank_exchange_workflow.py",
    }


def test_local_rerank_example_runs() -> None:
    result = _run_example_via_uv("examples/local_rerank_workflow.py")

    assert "rows_in= 2" in result.stdout
    assert "rows_out= 2" in result.stdout
    assert "top_doc_id= doc-a" in result.stdout
    assert "top_rank= 1" in result.stdout


def test_repo_search_example_exposes_help() -> None:
    result = _run_example_via_uv("examples/repo_search_workflow.py", "--help")

    assert "Run a host-backed repo-search analyzer workflow." in result.stdout
    assert "--query-text" in result.stdout
    assert "--path-prefix" in result.stdout


def test_custom_repo_search_example_exposes_help() -> None:
    result = _run_example_via_uv("examples/custom_repo_analyzer_workflow.py", "--help")

    assert "Run a host-backed repo-search workflow with a custom Python analyzer." in result.stdout
    assert "--query-text" in result.stdout
    assert "--path-prefix" in result.stdout


def test_host_backed_beta_smoke_example_exposes_help() -> None:
    result = _run_example_via_uv("examples/host_backed_repo_search_beta_smoke.py", "--help")

    assert "Run the full host-backed repo-search beta smoke path." in result.stdout
    assert "--build" in result.stdout
    assert "--mode {built_in,custom}" in result.stdout
    assert "--keep-workspace" in result.stdout
    assert "--workspace-root" in result.stdout
    assert "--repo-id" in result.stdout


def test_host_backed_rerank_beta_smoke_example_exposes_help() -> None:
    result = _run_example_via_uv("examples/host_backed_rerank_beta_smoke.py", "--help")

    assert "Run the full host-backed rerank beta smoke path." in result.stdout
    assert "--build" in result.stdout
    assert "--rerank-dimension" in result.stdout
    assert "--top-k" in result.stdout
    assert "--min-final-score" in result.stdout


def test_rerank_exchange_example_exposes_help() -> None:
    result = _run_example_via_uv("examples/rerank_exchange_workflow.py", "--help")

    assert "Run a host-backed rerank exchange workflow." in result.stdout
    assert "--top-k" in result.stdout
    assert "--min-final-score" in result.stdout


@pytest.mark.integration
def test_repo_search_example_runs_via_wendao_search_flight_server(tmp_path: Path) -> None:
    with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as sock:
        sock.bind(("127.0.0.1", 0))
        host, port = sock.getsockname()

    _run_rust_search_plane_seed_binary(tmp_path)
    process = _spawn_wendao_search_flight_server(host, port, tmp_path)
    try:
        result = _run_example_via_uv(
            "examples/repo_search_workflow.py",
            "--host",
            host,
            "--port",
            str(port),
            "--query-text",
            "alpha",
            "--path-prefix",
            "src/",
        )
    finally:
        _terminate_process(process)

    assert "query_text= alpha" in result.stdout
    assert "rows=" in result.stdout
    assert "top_path= src/" in result.stdout
    assert "top_rank= 1" in result.stdout


@pytest.mark.integration
def test_rerank_exchange_example_runs_via_runtime_wendao_flight_server() -> None:
    with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as sock:
        sock.bind(("127.0.0.1", 0))
        host, port = sock.getsockname()

    process = _spawn_wendao_runtime_flight_server(host, port)
    try:
        result = _run_example_via_uv(
            "examples/rerank_exchange_workflow.py",
            "--host",
            host,
            "--port",
            str(port),
            "--top-k",
            "2",
        )
    finally:
        _terminate_process(process)

    assert "rows_in= 2" in result.stdout
    assert "rows_out= 2" in result.stdout
    assert "top_doc_id= doc-0" in result.stdout
    assert "top_rank= 1" in result.stdout
    assert "top_final_score= 0.8" in result.stdout


@pytest.mark.integration
def test_host_backed_rerank_beta_smoke_example_runs() -> None:
    result = _run_example_via_uv(
        "examples/host_backed_rerank_beta_smoke.py",
        "--port",
        "0",
        "--top-k",
        "2",
    )

    assert "host= 127.0.0.1" in result.stdout
    assert "port=" in result.stdout
    assert "rerank_dimension= 3" in result.stdout
    assert "rows_in= 2" in result.stdout
    assert "rows_out= 2" in result.stdout
    assert "top_doc_id= doc-0" in result.stdout
    assert "top_rank= 1" in result.stdout
    assert "top_final_score= 0.8" in result.stdout


@pytest.mark.integration
def test_host_backed_beta_smoke_example_runs(tmp_path: Path) -> None:
    result = _run_example_via_uv(
        "examples/host_backed_repo_search_beta_smoke.py",
        "--port",
        "0",
        "--workspace-root",
        str(tmp_path / "beta-smoke-workspace"),
    )

    assert "workspace_root=" in result.stdout
    assert "mode= built_in" in result.stdout
    assert "keep_workspace= False" in result.stdout
    assert "host= 127.0.0.1" in result.stdout
    assert "port=" in result.stdout
    assert "query_text= alpha" in result.stdout
    assert "rows=" in result.stdout
    assert "top_path= src/" in result.stdout
    assert "top_rank= 1" in result.stdout


@pytest.mark.integration
def test_host_backed_beta_smoke_example_runs_in_custom_mode(tmp_path: Path) -> None:
    result = _run_example_via_uv(
        "examples/host_backed_repo_search_beta_smoke.py",
        "--mode",
        "custom",
        "--port",
        "0",
        "--workspace-root",
        str(tmp_path / "beta-smoke-custom-workspace"),
    )

    assert "mode= custom" in result.stdout
    assert "workspace_root=" in result.stdout
    assert "keep_workspace= False" in result.stdout
    assert "host= 127.0.0.1" in result.stdout
    assert "port=" in result.stdout
    assert "query_text= alpha" in result.stdout
    assert "rows=" in result.stdout
    assert "top_path= src/" in result.stdout
    assert "top_rank= 1" in result.stdout


@pytest.mark.integration
def test_host_backed_beta_smoke_example_keeps_temporary_workspace_when_requested() -> None:
    result = _run_example_via_uv(
        "examples/host_backed_repo_search_beta_smoke.py",
        "--port",
        "0",
        "--keep-workspace",
    )

    workspace_root_line = next(
        line for line in result.stdout.splitlines() if line.startswith("workspace_root= ")
    )
    workspace_root = Path(workspace_root_line.split("= ", 1)[1].strip())
    try:
        assert "keep_workspace= True" in result.stdout
        assert workspace_root.exists()
    finally:
        if workspace_root.exists():
            import shutil

            shutil.rmtree(workspace_root)


@pytest.mark.integration
def test_custom_repo_search_example_runs_via_wendao_search_flight_server(tmp_path: Path) -> None:
    with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as sock:
        sock.bind(("127.0.0.1", 0))
        host, port = sock.getsockname()

    _run_rust_search_plane_seed_binary(tmp_path)
    process = _spawn_wendao_search_flight_server(host, port, tmp_path)
    try:
        result = _run_example_via_uv(
            "examples/custom_repo_analyzer_workflow.py",
            "--host",
            host,
            "--port",
            str(port),
            "--query-text",
            "alpha",
            "--path-prefix",
            "src/",
        )
    finally:
        _terminate_process(process)

    assert "query_text= alpha" in result.stdout
    assert "rows=" in result.stdout
    assert "top_path= src/" in result.stdout
    assert "top_rank= 1" in result.stdout
