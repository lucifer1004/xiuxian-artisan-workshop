#!/usr/bin/env bash
set -euo pipefail

target_dir="${CARGO_TARGET_DIR:-/tmp/workspace-strict-proof}"

CARGO_TARGET_DIR="${target_dir}" cargo test -p omni-agent --test mcp_connect_startup
CARGO_TARGET_DIR="${target_dir}" cargo test -p omni-agent --test discover_cache_valkey_precedence
CARGO_TARGET_DIR="${target_dir}" cargo test -p omni-agent --test mcp_pool_hard_timeout
CARGO_TARGET_DIR="${target_dir}" cargo test -p omni-agent --test mcp_pool_reconnect
