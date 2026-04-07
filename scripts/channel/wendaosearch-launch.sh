#!/usr/bin/env bash

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "${SCRIPT_DIR}/../.." && pwd)"
source "${SCRIPT_DIR}/wendaosearch-common.sh"

if ! command -v julia >/dev/null 2>&1; then
  echo "Error: julia not found in PATH." >&2
  exit 1
fi

RUNTIME_DIR="$(wendaosearch_resolve_path "$PROJECT_ROOT" "$(wendaosearch_effective_runtime_dir)")"
PACKAGE_DIR="$(wendaosearch_package_dir "$PROJECT_ROOT")"
SCRIPT_PATH="$(wendaosearch_script_path "$PROJECT_ROOT")"
CONFIG_PATH="$(wendaosearch_resolve_path "$PROJECT_ROOT" "$(wendaosearch_effective_config)")"
JULIA_LOAD_PATH_VALUE="$(wendaosearch_effective_julia_load_path)"

mkdir -p "$RUNTIME_DIR"
test -f "$CONFIG_PATH" || {
  echo "Error: WendaoSearch config does not exist: $CONFIG_PATH" >&2
  exit 1
}
export JULIA_LOAD_PATH="$JULIA_LOAD_PATH_VALUE"

command=(julia "--project=${PACKAGE_DIR}" "$SCRIPT_PATH" "--config" "$CONFIG_PATH")

exec "${command[@]}"
