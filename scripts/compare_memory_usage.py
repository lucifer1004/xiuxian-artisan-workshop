#!/usr/bin/env python3
"""
Compare memory usage between upstream CLI and our implementation.
"""

import subprocess
import sys
import time
import os
from pathlib import Path

RSS_SCALE_FACTOR = 5


def resolve_prj_root() -> Path:
    """Resolve the project root from PRJ_ROOT or the script location."""
    raw_prj_root = os.environ.get("PRJ_ROOT")
    if raw_prj_root:
        return Path(raw_prj_root).expanduser().resolve()
    return Path(__file__).resolve().parent.parent


def get_process_rss_kb(pid: int) -> int:
    """Get RSS in KB using ps command."""
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
    """Get all descendant PIDs."""
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
    """Get total RSS for process and all descendants."""
    total = get_process_rss_kb(pid)
    for child_pid in get_child_processes(pid):
        total += get_process_rss_kb(child_pid)
    return total


def run_with_memory_monitor(cmd, label, max_rss_gb=15):
    """Run a command and monitor memory."""
    max_rss_kb = int(max_rss_gb * 1024 * 1024 / RSS_SCALE_FACTOR)

    print(f"\n=== {label} ===")
    print(f"Command: {' '.join(cmd)}")

    env = os.environ.copy()
    env["RUST_LOG"] = "debug"

    start_time = time.monotonic()
    proc = subprocess.Popen(
        cmd,
        env=env,
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

            # Read output (non-blocking)
            try:
                if hasattr(select, "select"):
                    readable, _, _ = select.select([proc.stdout], [], [], 0.05)
                    if readable:
                        line = proc.stdout.readline()
                        if line:
                            print(line, end="", flush=True)
            except:
                pass

            # Print memory every second
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
    print(f"\nDuration: {elapsed:.1f}s")
    print(f"Peak RSS: {peak_rss_gb:.2f} GB")

    return proc.returncode, peak_rss_gb


def main():
    prj_root = resolve_prj_root()
    model_root = prj_root / ".data/models/dots-ocr"
    test_image = prj_root / ".run/tmp/ocr-smoke.png"
    cli_path = os.environ.get(
        "DEEPSEEK_OCR_CLI",
        str(
            Path.home()
            / ".cargo/git/checkouts/deepseek-ocr.rs-83df09b3ffdef775/02b933d/target/release/deepseek-ocr-cli"
        ),
    )

    print(f"Project root: {prj_root}")
    print(f"Model root: {model_root}")

    # Run upstream CLI with --model flag to select dots-ocr-q6k
    cmd = [
        cli_path,
        "--model",
        "dots-ocr-q6k",
        "--device",
        "metal",
        "--prompt",
        "<image>\n<|grounding|>Convert this image to markdown.",
        "--image",
        str(test_image),
    ]

    returncode, peak = run_with_memory_monitor(cmd, "Upstream CLI (Metal)", max_rss_gb=15)
    print(f"\nUpstream CLI result: returncode={returncode}, peak={peak:.2f} GB")


if __name__ == "__main__":
    sys.exit(main())
