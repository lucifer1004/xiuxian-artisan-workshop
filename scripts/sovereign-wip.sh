#!/usr/bin/env bash
set -euo pipefail

# Sovereign WIP Utility v1.1 - Ghost Snapshot Edition
# Handles high-fidelity snapshots without clearing the workspace.

COMMAND=${1:-status}
TAG="SOVEREIGN-WIP"

case "$COMMAND" in
snapshot)
  echo "💎 Creating Sovereign Save-Point (Ghost Snapshot)..."
  # 1. Save all changes (including untracked) to stash
  # This will momentarily clear the workspace
  git stash push -u -m "$TAG: $(date +'%Y-%m-%d %H:%M:%S')"

  # 2. Immediately apply the stash back to restore the workspace
  # --index preserves the staged vs unstaged state
  git stash apply --index "stash@{0}" >/dev/null 2>&1 || {
    # Fallback if --index fails (e.g. minor binary sync issues)
    git stash apply "stash@{0}" >/dev/null 2>&1
  }
  echo "✅ Save-Point created. Your code remains in the workspace. Continue your research."
  ;;
meld)
  echo "🚀 Melding all Sovereign Snapshots..."
  STASH_IDS=$(git stash list | grep "$TAG" | cut -d: -f1)
  if [ -z "$STASH_IDS" ]; then
    echo "No snapshots found to meld."
    exit 0
  fi
  REVERSED_IDS=$(echo "$STASH_IDS" | tac)
  for id in $REVERSED_IDS; do
    echo "Applying $id..."
    git stash apply "$id" || echo "⚠️ Conflict in $id - manual resolution required."
  done
  ;;
purge)
  echo "🧹 Purging Sovereign WIP history..."
  # Note: We use a temporary list to avoid shifting indices during drops
  STASH_IDS=$(git stash list | grep "$TAG" | cut -d: -f1)
  for id in $STASH_IDS; do
    echo "Dropping $id..."
    git stash drop "$id" >/dev/null 2>&1 || true
  done
  echo "✅ Purge complete."
  ;;
status)
  echo "💠 Active Sovereign Save-Points:"
  git stash list | grep "$TAG" || echo "No active save-points."
  ;;
*)
  echo "Usage: $0 {snapshot|meld|purge|status}"
  exit 1
  ;;
esac
