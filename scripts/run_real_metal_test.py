#!/usr/bin/env python3
"""
Runner for real_metal test with both capacity check and runtime memory guard.

1. Uses capfox to check capacity before starting
2. Monitors memory during execution and kills if exceeded
"""

from __future__ import annotations

import os
import subprocess
import sys
import time
from pathlib import Path

# ps RSS is ~5x lower than Activity Monitor's Memory on macOS
RSS_SCALE_FACTOR = 5

# Find the test binary
PROJECT_ROOT = Path(__file__).resolve().parent.parent
TEST_BINARY_PATTERN = "llm_vision_deepseek_real_metal-"
CAPFOX_PATH = PROJECT_ROOT / ".run" / "capfox"


def find_test_binary() -> Path | None:
    """Find the real_metal test binary."""
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


def main() -> int:
    # Parse arguments
    use_cpu = False
    max_rss_gb = 10.0  # Default 10GB (Metal mode needs more)

    for arg in sys.argv[1:]:
        if arg == "--cpu":
            use_cpu = True
            max_rss_gb = 12.0  # CPU mode may need more RAM
        elif arg.startswith("--max-rss="):
            max_rss_gb = float(arg.split("=", 1)[1])
        elif arg in ("-h", "--help"):
            print(f"Usage: {sys.argv[0]} [--cpu] [--max-rss=GB]")
            print("  --cpu         Force CPU device (avoids Metal GPU memory)")
            print("  --max-rss=GB  Maximum RSS in GB (default: 10 for Metal, 12 for CPU)")
            print()
            print("Uses capfox for capacity check, then monitors memory at runtime.")
            return 0

    # Adjust for ps RSS (ps RSS is ~5x lower than Activity Monitor)
    max_rss_kb = int(max_rss_gb * 1024 * 1024 / RSS_SCALE_FACTOR)
    print(f"Max RSS (Activity Monitor): {max_rss_gb} GB")
    print(f"Max RSS (ps, scaled): {max_rss_kb / 1024:.0f} MB")

    # Find test binary
    binary = find_test_binary()
    if not binary:
        print("ERROR: Test binary not found.")
        print("Run: cargo build -p xiuxian-llm --features vision-dots-metal --tests")
        return 1

    print(f"Test binary: {binary}")

    # Check if test image exists
    test_image = PROJECT_ROOT / ".run/tmp/ocr-smoke.png"
    if not test_image.exists():
        print(f"ERROR: Test image not found: {test_image}")
        return 1

    print(f"Test image: {test_image}")

    # Check if capfox exists
    capfox = CAPFOX_PATH
    if not capfox.exists():
        print(f"WARNING: capfox not found at {capfox}")
        capfox = None

    # Test expects to be run from xiuxian-llm crate directory for path resolution
    test_cwd = PROJECT_ROOT / "packages" / "rust" / "crates" / "xiuxian-llm"

    env = os.environ.copy()
    env["RUST_BACKTRACE"] = "1"
    env["RUST_LOG"] = os.environ.get("RUST_LOG", "xiuxian_llm=debug,info")

    if use_cpu:
        env["XIUXIAN_VISION_DEVICE"] = "cpu"
        print("Device: CPU (forced)")

    # Build test command
    test_cmd = [
        str(binary),
        "test_real_metal_inference",
        "--test-threads=1",
        "--nocapture",
    ]

    # Phase 1: Capacity check with capfox (fail-open)
    if capfox:
        print()
        print("=== Phase 1: Capacity Check ===")
        if use_cpu:
            check_cmd = [
                str(capfox),
                "run",
                "--task",
                "deepseek_ocr_test",
                "--mem",
                "50",
                "--reason",
                "--",
                "true",  # Just check capacity, don't run
            ]
        else:
            check_cmd = [
                str(capfox),
                "run",
                "--task",
                "deepseek_ocr_metal_test",
                "--gpu",
                "80",
                "--vram",
                "60",
                "--mem",
                "30",
                "--reason",
                "--",
                "true",
            ]

        result = subprocess.run(check_cmd, env=env, cwd=test_cwd)
        if result.returncode == 75:
            print("=== CAPACITY DENIED ===")
            print("System does not have capacity for this test.")
            return 75
        # If capfox fails for other reasons, continue anyway (fail-open)

    # Phase 2: Run test with runtime memory guard
    print()
    print("=== Phase 2: Run Test with Memory Guard ===")
    print(f"Starting test (max RSS: {max_rss_gb} GB)...")
    print()

    start_time = time.monotonic()
    proc = subprocess.Popen(
        test_cmd,
        env=env,
        cwd=test_cwd,
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,  # Merge stderr into stdout
        text=True,
        bufsize=1,  # Line buffered
    )

    killed = False
    try:
        import select

        while proc.poll() is None:
            # Check memory
            total_rss_kb = get_total_rss_kb(proc.pid)
            total_rss_gb = total_rss_kb * RSS_SCALE_FACTOR / 1024 / 1024

            elapsed = time.monotonic() - start_time

            # Read and print output (non-blocking)
            try:
                # Use select to check if there's data to read
                if hasattr(select, "select"):
                    readable, _, _ = select.select([proc.stdout], [], [], 0.05)
                    if readable:
                        line = proc.stdout.readline()
                        if line:
                            print(line, end="", flush=True)
            except:
                pass

            # Print memory status every second
            if int(elapsed) != int(elapsed - 0.1):
                print(f"[{elapsed:.1f}s] RSS: {total_rss_gb:.2f} GB", flush=True)

            # Check memory limit
            if total_rss_kb > max_rss_kb:
                print(f"\n\n*** MEMORY LIMIT EXCEEDED: {total_rss_gb:.2f} GB > {max_rss_gb} GB ***")
                print("*** KILLING PROCESS ***")
                proc.kill()
                killed = True
                break

    except KeyboardInterrupt:
        print("\n*** INTERRUPTED ***")
        proc.kill()
        killed = True

    # Read any remaining output
    if proc.stdout:
        remaining = proc.stdout.read()
        if remaining:
            print(remaining, end="", flush=True)

    # Wait for process to finish
    if proc.poll() is None:
        proc.wait()

    exit_code = proc.returncode
    elapsed = time.monotonic() - start_time

    print()
    print(f"Exit code: {exit_code}")
    print(f"Duration: {elapsed:.1f}s")

    if killed:
        print("*** TEST WAS KILLED DUE TO MEMORY LIMIT ***")
        return 137

    if exit_code == 0:
        print("=== TEST PASSED ===")

    return exit_code


if __name__ == "__main__":
    sys.exit(main())
