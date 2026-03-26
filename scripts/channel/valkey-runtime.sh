#!/usr/bin/env bash

valkey_env_or_default() {
  local name="$1"
  local default="$2"
  local value="${!name:-}"

  if [ -n "$value" ]; then
    printf '%s' "$value"
  else
    printf '%s' "$default"
  fi
}

valkey_resolve_path() {
  local base="$1"
  local path="$2"

  if [ -z "$path" ]; then
    return 0
  fi

  case "$path" in
    /*) printf '%s' "$path" ;;
    *) printf '%s/%s' "$base" "$path" ;;
  esac
}

valkey_effective_port() {
  valkey_env_or_default VALKEY_PORT 6379
}

valkey_effective_host() {
  valkey_env_or_default VALKEY_HOST 127.0.0.1
}

valkey_effective_bind() {
  valkey_env_or_default VALKEY_BIND 0.0.0.0
}

valkey_effective_protected_mode() {
  if [ -n "${VALKEY_PROTECTED_MODE:-}" ]; then
    printf '%s' "$VALKEY_PROTECTED_MODE"
  fi
}

valkey_effective_db() {
  valkey_env_or_default VALKEY_DB 0
}

valkey_effective_runtime_dir() {
  valkey_env_or_default VALKEY_RUNTIME_DIR /run/valkey
}

valkey_effective_data_dir() {
  valkey_env_or_default VALKEY_DATA_DIR /data/valkey
}

valkey_effective_pidfile() {
  if [ -n "${VALKEY_PIDFILE:-}" ]; then
    printf '%s' "$VALKEY_PIDFILE"
    return 0
  fi

  printf '%s/valkey-%s.pid' "$(valkey_effective_runtime_dir)" "$(valkey_effective_port)"
}

valkey_effective_logfile() {
  if [ -n "${VALKEY_LOGFILE:-}" ]; then
    printf '%s' "$VALKEY_LOGFILE"
  fi
}

valkey_effective_tcp_backlog() {
  valkey_env_or_default VALKEY_TCP_BACKLOG 128
}

valkey_effective_daemonize() {
  valkey_env_or_default VALKEY_DAEMONIZE no
}

valkey_effective_startup_initial_delay_seconds() {
  valkey_env_or_default VALKEY_STARTUP_INITIAL_DELAY_SECONDS 5
}

valkey_effective_startup_period_seconds() {
  valkey_env_or_default VALKEY_STARTUP_PERIOD_SECONDS 2
}

valkey_effective_startup_failure_threshold() {
  valkey_env_or_default VALKEY_STARTUP_FAILURE_THRESHOLD 30
}

valkey_effective_url() {
  printf 'redis://%s:%s/%s' \
    "$(valkey_effective_host)" \
    "$(valkey_effective_port)" \
    "$(valkey_effective_db)"
}
