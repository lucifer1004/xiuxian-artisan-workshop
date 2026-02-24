#!/usr/bin/env bash
set -euo pipefail

echo "Checking Rust crate lint inheritance..."
missing="$(
  for f in packages/rust/crates/*/Cargo.toml; do
    if ! awk '
      BEGIN { in_lints = 0; has_workspace_true = 0 }
      /^\[lints\]/ { in_lints = 1; next }
      /^\[/ { in_lints = 0 }
      in_lints && /workspace[[:space:]]*=[[:space:]]*true/ { has_workspace_true = 1 }
      END { exit(has_workspace_true ? 0 : 1) }
    ' "$f"; then
      echo "$f"
    fi
  done
)"
if [ -n "$missing" ]; then
  echo "Missing [lints] workspace = true in:"
  echo "$missing"
  exit 1
fi

echo "All Rust crates inherit workspace lint policy."
