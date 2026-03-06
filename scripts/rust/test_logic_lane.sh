#!/usr/bin/env bash
set -euo pipefail

NEXTTEST_GUARD_LABEL="${NEXTTEST_GUARD_LABEL:-logic-lane}" \
  NEXTTEST_GUARD_MAX_RSS_GB="${NEXTTEST_GUARD_MAX_RSS_GB:-6}" \
  just nextest-guarded \
  --workspace \
  --exclude xiuxian-core-rs \
  -E 'not test(vision) and not test(ocr)' \
  "$@"
