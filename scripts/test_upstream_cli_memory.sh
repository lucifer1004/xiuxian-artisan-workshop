#!/bin/bash
# Test upstream CLI memory usage with different configurations

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PRJ_ROOT="${PRJ_ROOT:-$(cd "$SCRIPT_DIR/.." && pwd)}"

CLI_PATH="${DEEPSEEK_OCR_CLI:-$HOME/.cargo/git/checkouts/deepseek-ocr.rs-83df09b3ffdef775/02b933d/target/release/deepseek-ocr-cli}"
TEST_IMAGE="$PRJ_ROOT/.run/tmp/ocr-smoke.png"
MODEL_ROOT="$PRJ_ROOT/.data/models/dots-ocr"
PROMPT="<image>\n<|grounding|>Convert this image to markdown."

echo "=== Testing Upstream CLI Memory Usage ==="
echo ""
echo "Note: Watching memory with 'top' in another terminal is recommended."
echo "Press Enter to start CPU test..."
read

echo ""
echo "=== Test 1: CPU mode (default) ==="
echo "Running upstream CLI with CPU..."
/usr/bin/time -l "$CLI_PATH" \
  --device cpu \
  --weights "$MODEL_ROOT/model.safetensors.index.json" \
  --model-config "$MODEL_ROOT/config.json" \
  --tokenizer "$MODEL_ROOT/tokenizer.json" \
  --prompt "$PROMPT" \
  --image "$TEST_IMAGE" \
  2>&1 || echo "Test failed or not supported"

echo ""
echo "=== Test 2: Check if --snapshot flag exists ==="
"$CLI_PATH" --help 2>&1 | grep -i snapshot || echo "No snapshot option in help"

echo ""
echo "=== Done ==="
