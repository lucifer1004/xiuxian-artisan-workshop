#!/usr/bin/env bash
# Compatibility wrapper: use Python implementation for Discord ACL black-box probes.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
exec python3 "${SCRIPT_DIR}/test_omni_agent_discord_acl_events.py" "$@"
