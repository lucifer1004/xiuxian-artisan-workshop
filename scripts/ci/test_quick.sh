#!/usr/bin/env bash
set -euo pipefail

uv run pytest packages/python/foundation/tests/ packages/python/core/tests/ packages/python/agent/tests/unit/cli/ -q --tb=short

uv run pytest \
  scripts/test_render_wendao_ppr_rollout_status.py \
  scripts/test_wendao_ppr_rollout_ci.py \
  scripts/test_gate_wendao_ppr_ci.py \
  scripts/test_render_wendao_gate_status_summary.py \
  scripts/test_render_wendao_gate_line.py \
  scripts/test_render_wendao_rollout_gate_line.py \
  scripts/test_no_inline_python_exec_patterns.py \
  scripts/channel/test_epoch_millis.py \
  scripts/channel/test_config_resolver_compat.py \
  scripts/channel/test_resolve_mcp_port_from_settings.py \
  scripts/channel/test_check_mcp_health.py \
  scripts/channel/test_generate_secret_token.py \
  scripts/channel/test_read_telegram_setting.py \
  scripts/channel/test_extract_ngrok_public_url.py \
  scripts/rust/test_resolve_libpython_path.py \
  packages/ncl/scripts/test_print_skill_summary.py \
  -q --tb=short

(
  cd packages/python/mcp-server
  uv run pytest tests/ -q --tb=short --ignore=tests/integration/test_sse.py \
    --ignore=tests/unit/test_interfaces.py \
    --ignore=tests/unit/test_types.py \
    --ignore=tests/unit/test_transport/test_sse.py
)
