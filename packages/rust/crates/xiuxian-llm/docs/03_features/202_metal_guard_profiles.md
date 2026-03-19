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

### `deepseek_metal_smoke_12g_safe384_telegram_phrase_metal_fast_eager_shared_native`

Retained phrase-level follow-up on top of the same accepted `metal_fast` guarded shape. This keeps
the same `12 GB` budget but raises the semantic bar again from a single visible word to the visible
phrase `Telegram OCR`.

Current status:

- retained as the strongest short-form quality gate that is still practical under the current
  accepted Metal head
- stays within the `12 GB` guard and completes without reintroducing the long low-RSS tail
- depends on the retained prompt-aware first-token preference, now generalized to support
  `visible phrase ...` prompts by anchoring the first visible token
- under the current implementation the profile-backed rerun returns `Telegram OCR` in about `59s`

Primary evidence:

- `.run/tmp/downstream_deepseek_metal_telegram_phrase_profile_12g_v1.log`
- `.run/tmp/downstream_deepseek_metal_telegram_phrase_probe_12g_v1.log`

### `deepseek_metal_smoke_12g_safe384_sidecar_line_metal_fast_eager_shared_native`

Retained line-level follow-up on top of the same accepted `metal_fast` guarded shape. This keeps
the same `12 GB` budget but raises the semantic bar from a short phrase gate to a short structured
line fragment: `sidecar health check`.

Current status:

- retained as the lower-bound structured-line gate under the accepted `metal_fast` head
- promoted into TOML and harness coverage in this pass
- the matching manual probe stays within the `12 GB` guard without reopening the old low-RSS tail
- uses the same prompt-aware first-token path as the retained `Telegram` and `Telegram OCR` gates,
  but targets the visible phrase `managed sidecar health check`
- the retained manual probe returns `Managed sidecar health check.` in about `94s`
- one fresh profile-backed rerun also completed successfully under the same `12 GB` guard and
  returned `Managed sidecar health check.` in about `162s`
- a later file-capture rerun for the same profile was denied by `capfox` with `cpu_overload`, so
  repeatability is currently sensitive to ambient workspace load even though the gate has now been
  observed passing as a real profile-backed Metal run

Primary evidence:

- `.run/tmp/downstream_deepseek_metal_sidecar_line_profile_12g_v1.observed.log`
- `.run/tmp/downstream_deepseek_metal_sidecar_line_profile_12g_v1.log`
- `.run/tmp/downstream_deepseek_metal_sidecar_line_probe_12g_v1.log`

### `deepseek_metal_smoke_12g_safe384_managed_sidecar_line_metal_fast_eager_shared_native`

Retained stronger follow-up on the same accepted `metal_fast` guarded shape. This keeps the same
`12 GB` budget and the same visible phrase target, but now requires the leading token as well:
`Managed sidecar health check`.

Current status:

- retained as a profile-backed stronger line gate under the accepted `metal_fast` head
- uses the same guarded shape as the existing sidecar and memory-line probes, with
  `max_new_tokens = 6`
- the direct profile-backed rerun completed successfully under the `12 GB` guard and returned
  `Managed sidecar health check.` in about `80s`
- this uses the new CLI prompt/substr override path in the harness, so the evidence is a direct
  file-backed run rather than an observed side note

Primary evidence:

- `.run/tmp/downstream_deepseek_metal_managed_sidecar_line_profile_12g_v2.log`

### `deepseek_metal_smoke_12g_safe384_memory_line_metal_fast_eager_shared_native`

Configured memory-line follow-up on top of the same accepted `metal_fast` guarded shape.

Current status:

- profile default (`max_new_tokens = 6`) is no longer retained under the same `12 GB` guard after
  local `target/debug` refresh; repeated reruns now exceed the guard before decode completes
- narrowed decode budget (`max_new_tokens = 2`) produced two passing runs, but subsequent reruns
  in the same workspace snapshot failed again under the same `12 GB` guard
- phase-isolation confirms this wall is not decode-only in the current snapshot: both
  `--phase=prewarm` and `--phase=load` exceed the same `12 GB` guard, and a `13 GB` load retry
  also exceeds guard
- forcing `XIUXIAN_VISION_LAZY_MOE_EXPERTS=1` in `--phase=prewarm` does not restore `12 GB`
  stability in this snapshot
- `max_new_tokens = 3` still reintroduces the same guard breach; the memory-line branch is
  currently exploratory only and not retained

Primary evidence:

- `.run/tmp/downstream_deepseek_metal_memory_line_probe_12g_v1.log`
- `.run/tmp/downstream_deepseek_metal_memory_line_profile_12g_v3.log`
- `.run/tmp/downstream_deepseek_metal_memory_line_profile_default_12g_v1.log`
- `.run/tmp/downstream_deepseek_metal_memory_line_probe_12g_v4_after_rebuild.log`
- `.run/tmp/downstream_deepseek_metal_memory_line_profile_default_12g_tokens2_v1.log`
- `.run/tmp/downstream_deepseek_metal_memory_line_profile_default_12g_tokens2_v2.log`
- `.run/tmp/downstream_deepseek_metal_memory_line_profile_default_12g_tokens2_v3.log`
- `.run/tmp/downstream_deepseek_metal_memory_line_profile_default_12g_tokens3_v1.log`
- `.run/tmp/downstream_deepseek_metal_memory_line_profile_default_12g_v2_after_toml.log`
- `.run/tmp/downstream_deepseek_metal_memory_line_profile_default_12g_v3_after_toml.log`
- `.run/tmp/downstream_deepseek_metal_memory_line_profile_default_12g_prewarm_v1.log`
- `.run/tmp/downstream_deepseek_metal_memory_line_profile_default_12g_prewarm_lazy_moe_v1.log`
- `.run/tmp/downstream_deepseek_metal_memory_line_profile_default_12g_load_v1.log`
- `.run/tmp/downstream_deepseek_metal_memory_line_profile_default_13g_load_v1.log`

### GPU runner surface

The shared guarded runner in `scripts/run_real_metal_test.py` is now GPU-backend aware rather than
hard-wired to CPU-versus-Metal only.

Current status:

- the runner now accepts `--cuda` in addition to the retained default Metal path and the older
  `--cpu` fallback
- CUDA currently reuses the Metal-style GPU guard defaults unless explicit CUDA guard values are
  added to `test_guard`
- the shared ignored test surface now exposes both `test_real_metal_inference` and
  `test_real_cuda_inference` from the same file-backed GPU harness
- this is execution-surface readiness only; there is still no retained local CUDA evidence in this
  workspace snapshot

### Exploratory month-value probes

Two shorter month-value probes were run with manual prompt overrides on top of the canonical
accepted-head profile:

- no-cache month probe: `.run/tmp/downstream_deepseek_metal_month_value_probe_12g_v1.log`
- cache-enabled month probe: `.run/tmp/downstream_deepseek_metal_month_value_cache_probe_12g_v1.log`

Current status:

- both stayed within the `12 GB` guard, so they do not reopen the memory problem
- neither completed within a practical smoke-test window
- cache did not make the month-value probe fast enough to become the next gate

### Exploratory structured-field probes

Two more bounded structured-field probes were run after extending the retained prompt-aware first
visible-token anchor so punctuation-led phrases can still contribute an alphanumeric first anchor.

Primary evidence:

- visible year probe: `.run/tmp/downstream_deepseek_metal_year_word_probe_12g_v1.log`
- visible invoice-number probe: `.run/tmp/downstream_deepseek_metal_invoice_number_probe_12g_v3.log`
- visible amount-line probe: `.run/tmp/downstream_deepseek_metal_amount_line_probe_12g_v1.log`

Current status:

- both probes stay within the `12 GB` guard after the initial load/prefill window
- neither is practical enough to promote into a profile-backed gate yet
- the visible year probe still settles into the same long low-RSS tail seen in the earlier
  year-value probe
- the visible invoice-number probe also falls into a low-RSS tail, so the retained punctuation
  anchor is not sufficient by itself to make full structured numeric fields practical
- the amount-line probe gets farther than the numeric-only amount probe because it starts with a
  stable alphabetic token, but it still settles into a long high-RSS plateau around `9 GB` for
  more than `170s` without producing a useful OCR result, so it remains exploratory only
- the visible line `Hello from Telegram OCR.` is also exploratory only: it is allowed under the
  shared `12 GB` guard, but after an initial `~9.4 GB` decode window it drops into a long
  low-RSS tail around `1.54 GB` for more than `100s` without converging to a retained result
- the title-line candidate `Omni OCR smoke test` now has a clean non-capacity run:
  `.run/tmp/downstream_deepseek_metal_title_line_probe_12g_v2.log` passed `capfox` and stayed
  within the `12 GB` guard, but fell into the same long low-RSS tail around `1.67 GB` for more
  than `100s` without converging to a retained result, so it remains exploratory-only

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
- `.run/tmp/downstream_deepseek_metal_accepted_head_load_15g_stage_trace_v1.log`
- `.run/tmp/downstream_deepseek_metal_accepted_head_infer_13g_stage_trace_v1.log`

Latest accepted-head attribution (2026-03-18 refresh):

- the trace still reliably crosses `deepseek.load.weights_mmap.completed`,
  `deepseek.load.deferred_moe_source.completed`, and `deepseek.load.language.started`
- the canonical accepted-head profile now passes with PTY tracing for both widened diagnostics:
  `--phase=load --max-rss=15` and `--phase=infer --max-rss=13`
- load reaches `deepseek.language.weights_ready` with observed peak RSS around `9.10 GB`
- infer reaches `deepseek.language.weights_ready`, `xiuxian.decode.started`, and
  `ocr_engine.decode.generate.completed`, with observed peak RSS around `10.88 GB` and final OCR
  output `0`
- no guard kill occurs in this refreshed canonical run pair; earlier layer-5 kill attribution
  remains historical evidence for older snapshots and exploratory branches

Structured year-token follow-up (2026-03-18 exploratory):

- two guarded retries still fail quickly even with `max_new_tokens=1` and the same accepted-head
  profile
- `--phase=infer --max-rss=12` exits on guard at `12.25 GB` for the longer structured prompt
  variant
- `--phase=infer --max-rss=12` exits on guard at `12.80 GB`, and `--max-rss=13` exits on guard at
  `13.39 GB`, for the shorter `Return year.` prompt variant
- a widened `--max-rss=15` short-prompt run does not provide a retained success signal yet; it
  drifts into a long low-RSS tail (`~2.20 GB`) without a practical completion boundary before
  manual stop
- this branch remains exploratory-only and is not promoted into the retained quality ladder

Evidence:

- `.run/tmp/downstream_deepseek_metal_year_token_probe_12g_v1.log`
- `.run/tmp/downstream_deepseek_metal_year_token_short_prompt_12g_v1.log`
- `.run/tmp/downstream_deepseek_metal_year_token_short_prompt_13g_v1.log`
- `.run/tmp/downstream_deepseek_metal_year_token_short_prompt_15g_v1.log`

Structured day-token follow-up (2026-03-18 exploratory):

- a new compact structured-field probe (`day token 09`, still `max_new_tokens=1`) does not open a
  retained window on the same accepted head
- `--phase=infer --max-rss=12` exits on guard at `12.27 GB` after about `31.8s`
- `--phase=infer --max-rss=13` exits on guard at `13.11 GB` after about `26.4s`
- a widened `--max-rss=15` retry clears the early spike (around `10.91 GB`), then falls into a
  long low-RSS tail at `~2.94 GB`; no completion boundary appears in-log and the run was manually
  stopped after sustained plateau
- this branch is exploratory-only and is not promoted into the retained quality ladder

Evidence:

- `.run/tmp/downstream_deepseek_metal_day_token_probe_12g_v1.log`
- `.run/tmp/downstream_deepseek_metal_day_token_probe_13g_v1.log`
- `.run/tmp/downstream_deepseek_metal_day_token_probe_15g_v1.log`

## Evidence Rule

Every profile change should be accompanied by:

1. A guarded run log under `.run/tmp`
2. The exact effective env overrides used by the script
3. A short note describing whether the run reached load, prefill, or decode before failure
4. If stage tracing is enabled, the log should record the last visible stage before kill
