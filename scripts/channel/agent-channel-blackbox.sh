#!/usr/bin/env bash
# Compatibility wrapper: use Python implementation for black-box webhook probe.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

if [ -n "${PYTHON_BIN:-}" ]; then
  exec "${PYTHON_BIN}" "${SCRIPT_DIR}/agent_channel_blackbox.py" "$@"
fi

if command -v uv >/dev/null 2>&1; then
  exec uv run python "${SCRIPT_DIR}/agent_channel_blackbox.py" "$@"
fi

exec python3 "${SCRIPT_DIR}/agent_channel_blackbox.py" "$@"
