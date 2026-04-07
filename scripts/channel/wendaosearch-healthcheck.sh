#!/usr/bin/env bash

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "${SCRIPT_DIR}/../.." && pwd)"
source "${SCRIPT_DIR}/wendaosearch-common.sh"

if ! command -v python3 >/dev/null 2>&1; then
  echo "Error: python3 not found in PATH." >&2
  exit 1
fi

HOST="$(wendaosearch_effective_host "$PROJECT_ROOT")"
PORT="$(wendaosearch_effective_port "$PROJECT_ROOT")"

python3 - "$HOST" "$PORT" <<'PY'
import socket
import sys

host = sys.argv[1]
port = int(sys.argv[2])
with socket.create_connection((host, port), timeout=2):
    pass
PY
