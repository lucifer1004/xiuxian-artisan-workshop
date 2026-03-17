# Project Anchor V1: DeepSeek OCR Runtime Parity Before Optimization

## Objective

Keep `xiuxian-llm` aligned with upstream `deepseek-ocr` behavior first, then optimize only after the guarded CPU and Metal paths are reproducible and explained.

## Hard Constraints

- Do not modify `candle-core` while the current bottleneck can still be isolated inside vendored `deepseek-ocr`.
- Keep configuration resolution stable and auditable through TOML-backed profiles.
- Treat guarded real-device runs as evidence-producing probes, not as ad hoc experiments.

## Current Scope

- DeepSeek OCR runtime integration in `xiuxian-llm`
- Vendored `deepseek-ocr` investigation for model-side Metal memory pressure
- Guarded profile execution through `scripts/run_real_metal_test.py`

## Current Working Rule

When a guarded run disagrees with a hypothesis, prefer the log and move the explanation. Do not preserve an optimization theory once the staged traces disagree with it.
