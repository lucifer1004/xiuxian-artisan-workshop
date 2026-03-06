#!/usr/bin/env bash
set -euo pipefail

XIUXIAN_VISION_STUB="${XIUXIAN_VISION_STUB:-1}" \
  NEXTTEST_GUARD_LABEL="${NEXTTEST_GUARD_LABEL:-vision-smoke}" \
  NEXTTEST_GUARD_MAX_RSS_GB="${NEXTTEST_GUARD_MAX_RSS_GB:-10}" \
  just nextest-guarded \
  -p xiuxian-llm \
  --test llm_vision \
  --test llm_vision_deepseek_runtime_unit \
  --test llm_vision_deepseek_config_unit \
  --test llm_vision_deepseek_weights_path_unit \
  "$@"
