#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(git rev-parse --show-toplevel 2>/dev/null || pwd)"
cd "${ROOT_DIR}"

echo "[ocr-sim] running OCR timeout recovery probes (no webhook/agent runtime required)"
cargo nextest run -p xiuxian-daochang --test llm
echo "[ocr-sim] success: OCR timeout no longer leaves gate stuck in busy state"
