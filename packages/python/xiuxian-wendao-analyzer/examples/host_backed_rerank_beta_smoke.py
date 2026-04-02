from __future__ import annotations

import argparse
import os
import socket
import subprocess
import time
from pathlib import Path


def package_root() -> Path:
    return Path(__file__).resolve().parents[1]


def project_root() -> Path:
    env_root = os.environ.get("PRJ_ROOT")
    if env_root:
        return Path(env_root)
    return Path(__file__).resolve().parents[4]


def default_runtime_server_binary() -> Path:
    return (
        project_root()
        / ".cache"
        / "pyflight-rust-contract-target"
        / "debug"
        / "wendao_flight_server"
    )


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Run the full host-backed rerank beta smoke path.",
    )
    parser.add_argument("--build", action="store_true")
    parser.add_argument("--host", default="127.0.0.1")
    parser.add_argument("--port", type=int, default=8816)
    parser.add_argument("--schema-version", default="v2")
    parser.add_argument("--rerank-dimension", type=int, default=3)
    parser.add_argument("--top-k", type=int, default=2)
    parser.add_argument("--min-final-score", type=float, default=None)
    return parser.parse_args()


def run_checked(
    command: list[str], *, cwd: Path, env: dict[str, str] | None = None
) -> subprocess.CompletedProcess[str]:
    result = subprocess.run(
        command,
        cwd=cwd,
        check=False,
        capture_output=True,
        text=True,
        env=env,
    )
    if result.returncode != 0:
        raise RuntimeError(
            "command failed:\n"
            f"command: {' '.join(command)}\n"
            f"stdout:\n{result.stdout}\n"
            f"stderr:\n{result.stderr}"
        )
    return result


def ensure_runtime_binary(*, build: bool) -> Path:
    server_binary = Path(
        os.environ.get("WENDAO_RUNTIME_SERVER_BINARY", str(default_runtime_server_binary()))
    )

    if build:
        run_checked(
            [
                "direnv",
                "exec",
                ".",
                "cargo",
                "build",
                "-p",
                "xiuxian-wendao-runtime",
                "--features",
                "julia",
                "--bin",
                "wendao_flight_server",
            ],
            cwd=project_root(),
        )

    if not server_binary.exists():
        raise RuntimeError(
            f"missing runtime server binary: {server_binary}\n"
            "Run with --build or set WENDAO_RUNTIME_SERVER_BINARY."
        )
    return server_binary


def choose_port(host: str, requested_port: int) -> int:
    if requested_port != 0:
        return requested_port
    with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as sock:
        sock.bind((host, 0))
        return int(sock.getsockname()[1])


def spawn_runtime_server(
    server_binary: Path,
    *,
    host: str,
    port: int,
    schema_version: str,
    rerank_dimension: int,
) -> subprocess.Popen[str]:
    process = subprocess.Popen(
        [
            str(server_binary),
            f"{host}:{port}",
            f"--schema-version={schema_version}",
            f"--rerank-dimension={rerank_dimension}",
        ],
        cwd=project_root(),
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
            raise RuntimeError(f"runtime server exited before readiness:\n{stderr}")
    raise RuntimeError("timed out waiting for runtime server readiness")


def terminate_process(process: subprocess.Popen[str]) -> None:
    process.terminate()
    try:
        process.wait(timeout=10)
    except subprocess.TimeoutExpired:
        process.kill()
        process.wait(timeout=10)


def run_rerank_example(
    *,
    host: str,
    port: int,
    schema_version: str,
    top_k: int,
    min_final_score: float | None,
) -> subprocess.CompletedProcess[str]:
    command = [
        "uv",
        "run",
        "python",
        "examples/rerank_exchange_workflow.py",
        "--host",
        host,
        "--port",
        str(port),
        "--schema-version",
        schema_version,
        "--top-k",
        str(top_k),
    ]
    if min_final_score is not None:
        command.extend(["--min-final-score", str(min_final_score)])
    return run_checked(command, cwd=package_root())


def main() -> None:
    args = parse_args()
    host = args.host
    port = choose_port(host, args.port)
    server_binary = ensure_runtime_binary(build=args.build)

    process: subprocess.Popen[str] | None = None
    try:
        process = spawn_runtime_server(
            server_binary,
            host=host,
            port=port,
            schema_version=args.schema_version,
            rerank_dimension=args.rerank_dimension,
        )
        result = run_rerank_example(
            host=host,
            port=port,
            schema_version=args.schema_version,
            top_k=args.top_k,
            min_final_score=args.min_final_score,
        )
        print(f"host= {host}")
        print(f"port= {port}")
        print(f"rerank_dimension= {args.rerank_dimension}")
        print(result.stdout, end="")
    finally:
        if process is not None:
            terminate_process(process)


if __name__ == "__main__":
    main()
