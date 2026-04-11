from __future__ import annotations

import os
import signal
import subprocess
import sys
import time
from pathlib import Path


def _wait_for_file(path: Path, timeout_secs: float = 5.0) -> str:
    deadline = time.time() + timeout_secs
    while time.time() < deadline:
        if path.exists():
            return path.read_text(encoding="utf-8").strip()
        time.sleep(0.05)
    raise TimeoutError(f"timed out waiting for {path}")


def _spawn_listener(tmp_path: Path) -> tuple[subprocess.Popen[str], int]:
    port_file = tmp_path / "port.txt"
    listener = subprocess.Popen(
        [
            sys.executable,
            "-c",
            (
                "import pathlib, signal, socket, sys\n"
                "sock = socket.socket()\n"
                "sock.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)\n"
                "sock.bind(('127.0.0.1', 0))\n"
                "sock.listen()\n"
                "pathlib.Path(sys.argv[1]).write_text(str(sock.getsockname()[1]), encoding='utf-8')\n"
                "signal.signal(signal.SIGTERM, lambda *_: sys.exit(0))\n"
                "while True:\n"
                "    conn, _ = sock.accept()\n"
                "    conn.close()\n"
            ),
            str(port_file),
        ],
        text=True,
    )
    port = int(_wait_for_file(port_file))
    return listener, port


def test_wendaosearch_healthcheck_reports_owned_listener(tmp_path: Path) -> None:
    project_root = Path(__file__).resolve().parents[2]
    script_path = Path(__file__).resolve().with_name("wendaosearch-healthcheck.sh")
    pidfile = tmp_path / "run" / "wendaosearch-parser-summary.pid"
    pidfile.parent.mkdir(parents=True, exist_ok=True)

    listener, port = _spawn_listener(tmp_path)
    try:
        pidfile.write_text(f"{listener.pid}\n", encoding="utf-8")

        env = os.environ.copy()
        env["WENDAOSEARCH_SERVICE_NAME"] = "wendaosearch-parser-summary"
        env["WENDAOSEARCH_RUNTIME_DIR"] = str(tmp_path / "run")
        env["WENDAOSEARCH_PIDFILE"] = str(pidfile)
        env["WENDAOSEARCH_HOST"] = "127.0.0.1"
        env["WENDAOSEARCH_PORT"] = str(port)
        env["WENDAOSEARCH_PYTHON"] = sys.executable

        result = subprocess.run(
            ["bash", str(script_path)],
            cwd=project_root,
            env=env,
            capture_output=True,
            text=True,
            check=True,
        )

        assert result.stdout == ""
    finally:
        listener.terminate()
        listener.wait(timeout=5)


def test_wendaosearch_healthcheck_rejects_mismatched_listener_pid(tmp_path: Path) -> None:
    project_root = Path(__file__).resolve().parents[2]
    script_path = Path(__file__).resolve().with_name("wendaosearch-healthcheck.sh")
    pidfile = tmp_path / "run" / "wendaosearch-parser-summary.pid"
    pidfile.parent.mkdir(parents=True, exist_ok=True)

    listener, port = _spawn_listener(tmp_path)
    sleeper = subprocess.Popen([sys.executable, "-c", "import time; time.sleep(30)"])
    try:
        pidfile.write_text(f"{sleeper.pid}\n", encoding="utf-8")

        env = os.environ.copy()
        env["WENDAOSEARCH_SERVICE_NAME"] = "wendaosearch-parser-summary"
        env["WENDAOSEARCH_RUNTIME_DIR"] = str(tmp_path / "run")
        env["WENDAOSEARCH_PIDFILE"] = str(pidfile)
        env["WENDAOSEARCH_HOST"] = "127.0.0.1"
        env["WENDAOSEARCH_PORT"] = str(port)
        env["WENDAOSEARCH_PYTHON"] = sys.executable

        result = subprocess.run(
            ["bash", str(script_path)],
            cwd=project_root,
            env=env,
            capture_output=True,
            text=True,
            check=False,
        )

        assert result.returncode == 1
        assert "does not match the listener" in result.stderr
    finally:
        listener.terminate()
        listener.wait(timeout=5)
        sleeper.send_signal(signal.SIGTERM)
        sleeper.wait(timeout=5)


def test_wendaosearch_healthcheck_reports_missing_listener(tmp_path: Path) -> None:
    project_root = Path(__file__).resolve().parents[2]
    script_path = Path(__file__).resolve().with_name("wendaosearch-healthcheck.sh")
    pidfile = tmp_path / "run" / "wendaosearch-parser-summary.pid"
    pidfile.parent.mkdir(parents=True, exist_ok=True)

    sleeper = subprocess.Popen([sys.executable, "-c", "import time; time.sleep(30)"])
    try:
        pidfile.write_text(f"{sleeper.pid}\n", encoding="utf-8")

        env = os.environ.copy()
        env["WENDAOSEARCH_SERVICE_NAME"] = "wendaosearch-parser-summary"
        env["WENDAOSEARCH_RUNTIME_DIR"] = str(tmp_path / "run")
        env["WENDAOSEARCH_PIDFILE"] = str(pidfile)
        env["WENDAOSEARCH_HOST"] = "127.0.0.1"
        env["WENDAOSEARCH_PORT"] = "65429"
        env["WENDAOSEARCH_PYTHON"] = sys.executable

        result = subprocess.run(
            ["bash", str(script_path)],
            cwd=project_root,
            env=env,
            capture_output=True,
            text=True,
            check=False,
        )

        assert result.returncode == 1
        assert "is not listening" in result.stderr
    finally:
        sleeper.send_signal(signal.SIGTERM)
        sleeper.wait(timeout=5)
