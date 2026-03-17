# Path-Scoped Observation

## Runtime Entry Points

- `packages/rust/crates/xiuxian-llm/src/llm/vision/deepseek/runtime/mod.rs`
- `packages/rust/crates/xiuxian-llm/src/llm/vision/deepseek/native/engine/loader.rs`
- `packages/rust/crates/xiuxian-llm/src/llm/vision/deepseek/native/engine/core.rs`
- `packages/rust/crates/xiuxian-llm/src/runtime/executors/deepseek.rs`

## Vendored Upstream Focus

- `third_party/deepseek-ocr/crates/infer-deepseek/src/model/mod.rs`
- `third_party/deepseek-ocr/crates/infer-deepseek/src/transformer/block.rs`
- `third_party/deepseek-ocr/crates/infer-deepseek/src/transformer/weights.rs`
- `third_party/deepseek-ocr/crates/infer-deepseek/src/vision/clip.rs`

## Test and Guard Entry Points

- `packages/rust/crates/xiuxian-llm/tests/llm_vision_deepseek_real_cpu.rs`
- `packages/rust/crates/xiuxian-llm/tests/llm_vision_deepseek_real_metal.rs`
- `scripts/run_real_metal_test.py`
- `packages/rust/crates/xiuxian-llm/resources/config/vision_deepseek.toml`

## Current Evidence Files

- `.run/tmp/downstream_deepseek_metal_policy_fix_max1.log`
- `.run/tmp/downstream_deepseek_metal_policy_fix_max1_scatter.log`
- `.run/tmp/downstream_deepseek_metal_profile_infer_safe_vision_forced448.log`

## Observation Rule

Every new guarded Metal conclusion should reference:

1. The exact profile name
2. The exact log path under `.run/tmp`
3. The code path under `third_party/deepseek-ocr` or `xiuxian-llm` that produced the observation
