#!/usr/bin/env python3
"""
Runner for minimal model load test.
Directly tests upstream load_dots_model without our wrapper code.
"""

from __future__ import annotations

import os
import subprocess
import sys
import time
from pathlib import Path

RSS_SCALE_FACTOR = 5

PROJECT_ROOT = Path(__file__).resolve().parent.parent
TEST_BINARY_PATTERN = "llm_vision_deepseek_minimal_load-"


def find_test_binary() -> Path | None:
    deps_dir = PROJECT_ROOT / "target" / "debug" / "deps"
    if not deps_dir.exists():
        return None
    for f in deps_dir.iterdir():
        if f.name.startswith(TEST_BINARY_PATTERN) and f.is_file() and os.access(f, os.X_OK):
            if f.suffix in (".d", ".o", ".rmeta"):
                continue
            return f
    return None


def get_process_rss_kb(pid: int) -> int:
    try:
        result = subprocess.run(
            ["ps", "-o", "rss=", "-p", str(pid)],
            capture_output=True,
            text=True,
            check=False,
        )
        return int(result.stdout.strip()) if result.stdout.strip() else 0
    except (ValueError, OSError):
        return 0


def get_child_processes(ppid: int) -> list[int]:
    try:
        result = subprocess.run(
            ["pgrep", "-P", str(ppid)],
            capture_output=True,
            text=True,
            check=False,
        )
        children = []
        for line in result.stdout.strip().split("\n"):
            if line:
                child_pid = int(line)
                children.append(child_pid)
                children.extend(get_child_processes(child_pid))
        return children
    except (ValueError, OSError):
        return []


def get_total_rss_kb(pid: int) -> int:
    total = get_process_rss_kb(pid)
    for child_pid in get_child_processes(pid):
        total += get_process_rss_kb(child_pid)
    return total


def main() -> int:
    max_rss_gb = 15.0
    max_rss_kb = int(max_rss_gb * 1024 * 1024 / RSS_SCALE_FACTOR)
    print(f"Max RSS (Activity Monitor): {max_rss_gb} GB")

    binary = find_test_binary()
    if not binary:
        print("ERROR: Test binary not found.")
        print("Run: cargo build -p xiuxian-llm --features vision-dots-metal --tests")
        return 1

    print(f"Test binary: {binary}")

    test_cwd = PROJECT_ROOT / "packages" / "rust" / "crates" / "xiuxian-llm"

    env = os.environ.copy()
    env["RUST_BACKTRACE"] = "1"
    env["RUST_LOG"] = "debug"

    device = env.get("XIUXIAN_VISION_DEVICE", "metal")
    print(f"Device: {device}")

    test_cmd = [
        str(binary),
        "test_minimal_model_load",
        "--ignored",
        "--test-threads=1",
        "--nocapture",
    ]

    print(f"\n=== Running MINIMAL model load test ===")
    print(f"Command: {' '.join(test_cmd)}")
    print()

    start_time = time.monotonic()
    proc = subprocess.Popen(
        test_cmd,
        env=env,
        cwd=test_cwd,
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
        text=True,
        bufsize=1,
    )

    peak_rss_gb = 0
    killed = False

    try:
        import select

        while proc.poll() is None:
            total_rss_kb = get_total_rss_kb(proc.pid)
            total_rss_gb = total_rss_kb * RSS_SCALE_FACTOR / 1024 / 1024
            peak_rss_gb = max(peak_rss_gb, total_rss_gb)
            elapsed = time.monotonic() - start_time

            try:
                if hasattr(select, "select"):
                    readable, _, _ = select.select([proc.stdout], [], [], 0.05)
                    if readable:
                        line = proc.stdout.readline()
                        if line:
                            print(line, end="", flush=True)
            except:
                pass

            if int(elapsed) != int(elapsed - 0.1):
                print(f"[{elapsed:.1f}s] RSS: {total_rss_gb:.2f} GB", flush=True)

            if total_rss_kb > max_rss_kb:
                print(f"\n*** MEMORY LIMIT EXCEEDED: {total_rss_gb:.2f} GB > {max_rss_gb} GB ***")
                proc.kill()
                killed = True
                break
    except KeyboardInterrupt:
        proc.kill()
        killed = True

    if proc.stdout:
        remaining = proc.stdout.read()
        if remaining:
            print(remaining, end="", flush=True)

    if proc.poll() is None:
        proc.wait()

    elapsed = time.monotonic() - start_time
    print(f"\nExit code: {proc.returncode}")
    print(f"Duration: {elapsed:.1f}s")
    print(f"Peak RSS: {peak_rss_gb:.2f} GB")

    if killed:
        print("*** TEST WAS KILLED DUE TO MEMORY LIMIT ***")
        return 137

    return proc.returncode


if __name__ == "__main__":
    sys.exit(main())
