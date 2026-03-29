#!/usr/bin/env bash
# Compatibility wrapper: use Python tool-list observability probe.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
exec python3 "${SCRIPT_DIR}/test_xiuxian_daochang_tool_list_observability.py" "$@"
