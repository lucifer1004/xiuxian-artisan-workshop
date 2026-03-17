# DeepSeek OCR Runtime Ledger

:PROPERTIES:
:ID: llm-deepseek-runtime-ledger
:PARENT: [[../index.md]]
:STATUS: ACTIVE
:END:

## Effective Runtime Principles

- Default `DeepSeek OCR` parity must come before `Dots OCR` tuning.
- Guarded CPU and Metal profiles are the canonical way to compare upstream and downstream behavior.
- DeepSeek OCR configuration should be visible as an effective runtime log, not inferred from scattered defaults.

## Confirmed Runtime Fixes

### 1. Config Parity Is TOML-Backed

The effective DeepSeek OCR config now resolves from `vision_deepseek.toml` instead of relying on ad hoc per-run overrides alone.

### 2. Deferred Load Policy Must Survive Beyond Initial Model Load

Lazy MoE expert and lazy CLIP transformer materialization must use the same `LowPrecisionLoadPolicy` that was active during model load. If that policy is allowed to fall back to the upstream default after the initial load guard exits, later guarded runs become misleading.

### 3. Guarded Real-Device Runs Are Required Evidence

Unit tests can prove policy plumbing, but only guarded real CPU or Metal runs can show whether the model actually respects the intended memory budget.

## Active Runtime Risks

- Metal runs remain sensitive to allocator behavior and repeated F16-to-F32 expert materialization.
- A single guarded run is not enough to mark a profile as stable; repeated runs still need to converge.
- Experimental MoE execution changes must be treated as provisional until the guarded Metal result is repeatable.
