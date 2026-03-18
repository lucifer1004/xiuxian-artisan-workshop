# Metal Guard Profiles

:PROPERTIES:
:ID: llm-metal-guard-profiles
:PARENT: [[../index.md]]
:STATUS: ACTIVE
:END:

## Canonical Profile Source

`packages/rust/crates/xiuxian-llm/resources/config/vision_deepseek.toml`

This file is the single source of truth for DeepSeek OCR real-device guard profiles.

## Current Important Profiles

### `deepseek_metal_guard_12g`

Baseline guarded Metal profile for DeepSeek OCR. This profile is intentionally strict and exists to surface the first real memory wall.

### `deepseek_metal_guard_12g_safe448`

Reduced-vision guarded Metal profile used for current DeepSeek OCR memory investigation. This profile keeps:

- `image_size=448`
- `base_size=448`
- lazy MoE experts enabled
- lazy CLIP transformer layers enabled
- no cache for OCR decode

### `deepseek_metal_smoke_12g_safe384`

Conservative Metal smoke profile for runtime-stability verification. This profile keeps:

- `image_size=384`
- `base_size=384`
- `crop_mode=false`
- `max_new_tokens=1`
- `allow_empty_output=true`
- lazy MoE experts enabled
- lazy CLIP transformer layers enabled
- no cache for OCR decode

This profile is intended to verify that Metal inference can complete inside the `12 GB` guard even when the OCR result is too small to count as a quality pass.

Validated evidence:

- `.run/tmp/downstream_deepseek_metal_smoke_safe384_status0.log`

### `deepseek_metal_smoke_12g_safe384_digit1`

Exploratory Metal profile for the smallest non-empty OCR target. This profile keeps the same
runtime-stability settings as `deepseek_metal_smoke_12g_safe384`, but changes the prompt to ask
for exactly one visible digit and keeps `max_new_tokens=1`.

Current status:

- not yet validated under the `12 GB` guard
- intended only for prompt-side and first-token investigation
- should not replace the canonical empty-output smoke baseline

Primary evidence:

- `.run/tmp/downstream_deepseek_metal_smoke_safe384_digit1.log`
- `.run/tmp/downstream_deepseek_metal_smoke_safe320_digit1_probe.log`
- `.run/tmp/downstream_deepseek_metal_smoke_safe320_digit1_trace.log`

### `deepseek_metal_safe320_digit1_native_moe`

Exploratory Metal profile for reducing routed-expert memory pressure during the smallest non-empty
OCR probe. This profile keeps:

- `base_size=320`
- `image_size=320`
- `crop_mode=false`
- `max_new_tokens=1`
- `moe_expert_f32_compute=false`
- lazy MoE experts enabled
- lazy CLIP transformer layers enabled
- no cache for OCR decode

Current status:

- still above the `12 GB` guard
- useful as an experimental profile, not a passing smoke baseline

Primary evidence:

- `.run/tmp/downstream_deepseek_metal_safe320_digit1_native_moe.log`
- `.run/tmp/downstream_deepseek_metal_safe320_digit1_native_moe_release.log`

### `deepseek_metal_smoke_12g_safe384_digit1_native_inputs`

Exploratory Metal profile for keeping language input embeddings in the native model dtype during
prefill and first-token decode. This profile keeps the same shape as
`deepseek_metal_smoke_12g_safe384_digit1`, but adds:

- `promote_language_input_f32 = false`

Current status:

- improved the guarded non-empty probe, but still exceeded the `12 GB` guard
- should be treated as a useful research profile, not as a passing smoke baseline

Primary evidence:

- `.run/tmp/downstream_deepseek_metal_safe384_digit1_native_inputs.log`

### `deepseek_metal_smoke_12g_safe384_digit1_native_inputs_native_moe`

Exploratory combination profile for testing both native language-input dtype and native MoE expert
compute in the same guarded non-empty probe.

Current status:

- rejected as a follow-up profile shape for now
- exceeded the `12 GB` guard earlier than the native-input-only profile

Primary evidence:

- `.run/tmp/downstream_deepseek_metal_safe384_digit1_native_inputs_native_moe.log`

### `deepseek_metal_smoke_12g_safe384_digit1_native_inputs_native_attn`

Exploratory combination profile for testing both native language-input dtype and native prefill
attention in the same guarded non-empty probe.

Current status:

- improved the guarded non-empty probe slightly, but still exceeded the `12 GB` guard
- remains a research profile, not a passing smoke baseline

Primary evidence:

- `.run/tmp/downstream_deepseek_metal_safe384_digit1_native_inputs_native_attn.log`

### `deepseek_metal_smoke_12g_safe384_digit1_native_inputs_native_attn_native_combine`

Exploratory combination profile for testing native language-input dtype, native prefill attention,
and native MoE combine/scatter working dtype in the same guarded non-empty probe.

Current status:

- rejected as a follow-up profile shape for now
- exceeded the `12 GB` guard earlier than the native-attention profile

Primary evidence:

- `.run/tmp/downstream_deepseek_metal_safe384_digit1_native_inputs_native_attn_native_combine.log`

### `deepseek_metal_smoke_12g_safe384_digit1_native_inputs_native_attn_native_gate_inputs`

Exploratory combination profile for testing native language-input dtype, native prefill attention,
and native MoE gating-input matmul dtype in the same guarded single-digit probe.

Current status:

- retained as the canonical guarded Metal smoke profile for the current `metal_fast` DeepSeek OCR
  path
- after the retained digit-first follow-up, the profile now passes guarded `infer` at both
  `12 GB` and `13 GB`
- the retained behavior is still narrow and prompt-scoped: whitespace-only decoded candidates are
  skipped, `eos` is deferred until no visible candidate remains, and prompts that explicitly ask
  for exactly one visible digit now prefer single-digit visible candidates over other visible text
- the current smoke evidence returns a reproducible preview (`0`) under the `12 GB` guard, and
  the canonical profile now encodes that minimum semantic contract directly with
  `expected_substring = "0"`
- step-0 logits still rank whitespace token `6776` highest and `eos` second, so the retained
  semantic change is in first-token selection policy, not raw logits ordering

Primary evidence:

- `.run/tmp/downstream_deepseek_metal_safe384_digit1_native_inputs_native_attn_native_gate_inputs.log`
- `.run/tmp/downstream_deepseek_metal_safe384_digit1_native_inputs_native_attn_native_gate_inputs_post_visible_revert.log`
- `.run/tmp/downstream_deepseek_metal_first_visible_eos_deferred_infer_12g_v1.log`
- `.run/tmp/downstream_deepseek_metal_first_visible_eos_deferred_infer_13g_v1.log`
- `.run/tmp/downstream_deepseek_metal_first_visible_eos_deferred_infer_13g_emptytrace_v1.log`
- `.run/tmp/downstream_deepseek_metal_digit_first_canonical_12g_v5.log`
- `.run/tmp/downstream_deepseek_metal_digit_first_canonical_12g_v6.log`
- `.run/tmp/downstream_deepseek_metal_canonical_step0_logits_v5.json`

### `deepseek_metal_smoke_12g_safe384_amount_value_metal_fast_eager_shared_native`

Exploratory multi-token quality profile that keeps the same accepted `metal_fast` guarded shape,
but raises the semantic bar from a single visible digit to the amount value and requires the output
to contain `128.50`.

Current status:

- retained as an exploratory quality profile only
- it stays within the `12 GB` guard, so memory is no longer the limiting factor for this shape
- it is not suitable as the next smoke gate: the run took about `252s` and returned `No units.`
  instead of an amount substring

Primary evidence:

- `.run/tmp/downstream_deepseek_metal_amount_value_12g_v1.log`

### `deepseek_metal_smoke_12g_safe384_telegram_word_metal_fast_eager_shared_native`

Retained stronger-quality Metal profile that keeps the same accepted `metal_fast` guarded shape,
but raises the semantic bar from a single visible digit to the visible word `Telegram`.

Current status:

- retained as the next practical stronger-quality gate on top of the canonical one-digit smoke
- stays within the `12 GB` guard and completes in a practical smoke window
- depends on the retained prompt-aware first-token word preference; under the current implementation
  it returns `Telegram` in about `37s`

Primary evidence:

- `.run/tmp/downstream_deepseek_metal_telegram_profile_12g_v1.log`
- `.run/tmp/downstream_deepseek_metal_telegram_probe_12g_v2.log`

### Exploratory month-value probes

Two shorter month-value probes were run with manual prompt overrides on top of the canonical
accepted-head profile:

- no-cache month probe: `.run/tmp/downstream_deepseek_metal_month_value_probe_12g_v1.log`
- cache-enabled month probe: `.run/tmp/downstream_deepseek_metal_month_value_cache_probe_12g_v1.log`

Current status:

- both stayed within the `12 GB` guard, so they do not reopen the memory problem
- neither completed within a practical smoke-test window
- cache did not make the month-value probe fast enough to become the next gate

### Rejected short-field probes

Two additional short-field probes were explored on top of the canonical accepted head:

- early `Telegram` word probe before the retained prompt-aware word preference:
  `.run/tmp/downstream_deepseek_metal_telegram_probe_12g_v1.log`
- invoice suffix probe: `.run/tmp/downstream_deepseek_metal_invoice_suffix_probe_12g_v1.log`

Current status:

- the early `Telegram` probe no longer reflects the retained implementation and has been
  superseded by the passing `v2` evidence above
- the invoice suffix probe stayed well below the `12 GB` guard after the initial load/prefill
  spike, but it did not produce a useful OCR result inside a practical smoke window
- the invoice suffix probe settled into the same long low-RSS tail as the rejected month-value
  probes and was manually stopped instead of being promoted into a TOML profile

### Rejected Follow-Up: Visible-First Token Steering

An additional research profile briefly tried to skip a whitespace-only first token before final
normalization.

Current status:

- rejected and removed from the current tree
- repeated guarded runs stayed above the `12 GB` budget, with representative failures in the
  `12.1-12.3 GB` range
- the lighter sampling-helper tightening was retained, but the visible-first steering path itself
  was not justified by the guarded results
- a later retained fix narrowed the scope further by changing only first-token selection on the
  canonical profile and by deferring `eos` until visible candidates are exhausted; that retained
  landing supersedes this rejected branch and does not use a separate profile

Primary evidence:

- `.run/tmp/downstream_deepseek_metal_safe384_digit1_native_inputs_native_attn_native_gate_inputs_visible_first.log`

## Invocation Pattern

```bash
python3 scripts/run_real_metal_test.py \
  --phase=infer \
  --profile=deepseek_metal_guard_12g_safe448
```

```bash
python3 scripts/run_real_metal_test.py \
  --phase=infer \
  --profile=deepseek_metal_smoke_12g_safe384
```

## PTY-Backed Trace Workflow

When a guarded Metal run needs stage-level attribution, enable
`XIUXIAN_VISION_STAGE_TRACE_STDERR=1` and let `scripts/run_real_metal_test.py` switch the child
test binary onto PTY transport automatically.

This is now validated for the accepted `metal_fast + shared_native` head, because ordinary
guarded logs can otherwise die before the first useful model-stage line becomes visible.

Validated evidence:

- `.run/tmp/downstream_deepseek_metal_metal_fast_profile_shared_native_trace_pty_load_15g_v3.log`
- `.run/tmp/downstream_deepseek_metal_metal_fast_profile_shared_native_trace_pty_13g_v3.log`

Latest accepted-head attribution:

- the trace now reliably crosses `deepseek.load.weights_mmap.completed`
- the trace now reliably crosses `deepseek.load.deferred_moe_source.completed`
- the trace now reliably crosses `deepseek.load.language.started`
- widened `load` diagnostics now go well past the old mmap boundary and into per-layer load
  internals
- with the finer MoE and linear-loader traces, the accepted-head `load` wall is now attributable
  inside MoE expert linear materialization rather than at a generic layer boundary
- the latest representative widened `load` trace dies during layer `5` expert loading, with the
  last visible linear label inside `model.layers.5.mlp.experts.59.up_proj.weight`
- the guarded `infer` diagnostic reaches `deepseek.language.transformer_layer.start` for
  `layer_idx=5` after completing layer `4` before the `13 GB` guard kills the process

This means the active accepted-head wall is no longer "somewhere around mmap". The current bounded
investigation should stay inside `LinearWeights::load` for MoE expert projections.

## Evidence Rule

Every profile change should be accompanied by:

1. A guarded run log under `.run/tmp`
2. The exact effective env overrides used by the script
3. A short note describing whether the run reached load, prefill, or decode before failure
4. If stage tracing is enabled, the log should record the last visible stage before kill
