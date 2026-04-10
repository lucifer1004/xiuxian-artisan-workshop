#!/usr/bin/env bash

set -euo pipefail

wendaosearch_default_service_name() {
  printf '%s\n' "wendaosearch-parser-summary"
}

wendaosearch_default_runtime_dir() {
  printf '%s\n' ".run/wendaosearch"
}

wendaosearch_default_config() {
  printf '%s\n' ".data/WendaoSearch.jl/config/live/parser_summary.toml"
}

wendaosearch_default_script_name() {
  printf '%s\n' "run_parser_summary_service.jl"
}

wendaosearch_default_host() {
  printf '%s\n' "127.0.0.1"
}

wendaosearch_default_port() {
  printf '%s\n' "41081"
}

wendaosearch_default_mode() {
  printf '%s\n' "solver_demo"
}

wendaosearch_default_route_names() {
  printf '%s\n' "capability_manifest,structural_rerank,constraint_filter"
}

wendaosearch_effective_service_name() {
  printf '%s\n' "${WENDAOSEARCH_SERVICE_NAME:-$(wendaosearch_default_service_name)}"
}

wendaosearch_effective_runtime_dir() {
  printf '%s\n' "${WENDAOSEARCH_RUNTIME_DIR:-$(wendaosearch_default_runtime_dir)}"
}

wendaosearch_effective_config() {
  printf '%s\n' "${WENDAOSEARCH_CONFIG:-$(wendaosearch_default_config)}"
}

wendaosearch_effective_script_name() {
  printf '%s\n' "${WENDAOSEARCH_SCRIPT:-$(wendaosearch_default_script_name)}"
}

wendaosearch_config_value() {
  local root="$1"
  local field="$2"
  local config_path
  config_path="$(wendaosearch_resolve_path "$root" "$(wendaosearch_effective_config)")"
  python3 - "$config_path" "$field" <<'PY'
import sys
import tomllib

path = sys.argv[1]
field = sys.argv[2]
with open(path, "rb") as handle:
    config = tomllib.load(handle)
value = config.get(field)
if value is None:
    raise SystemExit(1)
if isinstance(value, list):
    print(",".join(str(item) for item in value))
else:
    print(value)
PY
}

wendaosearch_config_section_value() {
  local root="$1"
  local section="$2"
  local field="$3"
  local config_path
  config_path="$(wendaosearch_resolve_path "$root" "$(wendaosearch_effective_config)")"
  python3 - "$config_path" "$section" "$field" <<'PY'
import sys
import tomllib

path = sys.argv[1]
section = sys.argv[2]
field = sys.argv[3]
with open(path, "rb") as handle:
    config = tomllib.load(handle)
section_value = config.get(section)
if not isinstance(section_value, dict):
    raise SystemExit(1)
value = section_value.get(field)
if value is None:
    raise SystemExit(1)
if isinstance(value, list):
    print(",".join(str(item) for item in value))
else:
    print(value)
PY
}

wendaosearch_effective_host() {
  if [ -n "${WENDAOSEARCH_HOST:-}" ]; then
    printf '%s\n' "$WENDAOSEARCH_HOST"
    return 0
  fi
  if [ "$(wendaosearch_effective_script_name)" = "run_parser_summary_service.jl" ]; then
    wendaosearch_config_section_value "$1" "interface" "host" 2>/dev/null || \
      wendaosearch_config_value "$1" "host" 2>/dev/null || \
      wendaosearch_default_host
    return 0
  fi
  wendaosearch_config_value "$1" "host" 2>/dev/null || wendaosearch_default_host
}

wendaosearch_effective_port() {
  if [ -n "${WENDAOSEARCH_PORT:-}" ]; then
    printf '%s\n' "$WENDAOSEARCH_PORT"
    return 0
  fi
  if [ "$(wendaosearch_effective_script_name)" = "run_parser_summary_service.jl" ]; then
    wendaosearch_config_section_value "$1" "interface" "port" 2>/dev/null || \
      wendaosearch_config_value "$1" "port" 2>/dev/null || \
      wendaosearch_default_port
    return 0
  fi
  wendaosearch_config_value "$1" "port" 2>/dev/null || wendaosearch_default_port
}

wendaosearch_effective_mode() {
  if [ -n "${WENDAOSEARCH_MODE:-}" ]; then
    printf '%s\n' "$WENDAOSEARCH_MODE"
    return 0
  fi
  wendaosearch_config_value "$1" "mode" 2>/dev/null || wendaosearch_default_mode
}

wendaosearch_effective_route_name() {
  if [ -n "${WENDAOSEARCH_ROUTE_NAME:-}" ]; then
    printf '%s\n' "$WENDAOSEARCH_ROUTE_NAME"
    return 0
  fi
  wendaosearch_config_value "$1" "route_name" 2>/dev/null || printf '%s\n' ""
}

wendaosearch_effective_route_names() {
  if [ -n "${WENDAOSEARCH_ROUTE_NAMES:-}" ]; then
    printf '%s\n' "$WENDAOSEARCH_ROUTE_NAMES"
    return 0
  fi
  wendaosearch_config_value "$1" "route_names" 2>/dev/null || wendaosearch_default_route_names
}

wendaosearch_effective_julia_load_path() {
  printf '%s\n' "${WENDAOSEARCH_JULIA_LOAD_PATH:-@:@stdlib}"
}

wendaosearch_resolve_path() {
  local root="$1"
  local path="$2"
  if [[ $path == /* ]]; then
    printf '%s\n' "$path"
  else
    printf '%s\n' "$root/$path"
  fi
}

wendaosearch_package_dir() {
  local root="$1"
  wendaosearch_resolve_path "$root" ".data/WendaoSearch.jl"
}

wendaosearch_script_path() {
  local root="$1"
  local package_dir
  package_dir="$(wendaosearch_package_dir "$root")"
  printf '%s\n' "$package_dir/scripts/$(wendaosearch_effective_script_name)"
}
