#!/usr/bin/env bash
set -euo pipefail

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

"${script_dir}/xiuxian_daochang_embedding_role_perf_smoke.sh" \
  "30" \
  "12" \
  "64" \
  "8" \
  "250" \
  "900" \
  "20" \
  ".run/reports/xiuxian-daochang-embedding-role-perf-smoke.medium.json"
