#!/usr/bin/env bash
set -euo pipefail

target_dir="${CARGO_TARGET_DIR:-/tmp/workspace-strict-proof}"

CARGO_TARGET_DIR="${target_dir}" cargo check -p xiuxian-mcp
CARGO_TARGET_DIR="${target_dir}" cargo test -p xiuxian-mcp --no-run
