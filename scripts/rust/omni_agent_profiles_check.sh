#!/usr/bin/env bash
set -euo pipefail

target_dir="${CARGO_TARGET_DIR:-/tmp/workspace-strict-proof}"

CARGO_TARGET_DIR="${target_dir}" cargo check -p omni-agent
CARGO_TARGET_DIR="${target_dir}" cargo check -p omni-agent --no-default-features
CARGO_TARGET_DIR="${target_dir}" cargo test -p omni-agent --no-run
CARGO_TARGET_DIR="${target_dir}" cargo test -p omni-agent --no-run --no-default-features
