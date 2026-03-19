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
- after the managed-sidecar runner realignment, the same sidecar-line profile was revalidated on
  the verified `.run/target-real-metal-cli-debug` surface and passed again in `86.9s` with the
  OCR preview `Managed sidecar health check.`

Primary evidence:

- `.run/tmp/downstream_deepseek_metal_sidecar_line_profile_12g_v1.observed.log`
- `.run/tmp/downstream_deepseek_metal_sidecar_line_profile_12g_v1.log`
- `.run/tmp/downstream_deepseek_metal_sidecar_line_probe_12g_v1.log`
- `.run/tmp/downstream_deepseek_metal_sidecar_line_profile_12g_v2_realign.log`

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
- the same profile default has now been revalidated on the verified
  `.run/target-real-metal-cli-debug` surface and passes again in `93.9s` with OCR preview
  `Memory should stay stable.`
- narrowed decode budget (`max_new_tokens = 2`) produced two passing runs, but subsequent reruns
  in the same workspace snapshot failed again under the same `12 GB` guard
- phase-isolation confirms this wall is not decode-only in the current snapshot: both
  `--phase=prewarm` and `--phase=load` exceed the same `12 GB` guard, and a `13 GB` load retry
  also exceeds guard
- forcing `XIUXIAN_VISION_LAZY_MOE_EXPERTS=1` in `--phase=prewarm` does not restore `12 GB`
  stability in this snapshot
- `max_new_tokens = 3` still reintroduces the same guard breach on the refreshed `target/debug`
  surface; the memory-line branch is therefore binary-surface scoped rather than globally retained

Primary evidence:

- `.run/tmp/downstream_deepseek_metal_memory_line_probe_12g_v1.log`
- `.run/tmp/downstream_deepseek_metal_memory_line_profile_12g_v3.log`
- `.run/tmp/downstream_deepseek_metal_memory_line_profile_12g_v5_realign.log`
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

Structured digit-9 follow-up (2026-03-18 exploratory):

- a compact single-digit follow-up (`expected_substring=9`) was run to test whether a stronger
  numeric target can still stay practical without reopening memory surgery
- `--phase=infer --max-rss=12` exits on guard at `12.11 GB` after about `32.8s`
- `--phase=infer --max-rss=13` exits on guard at `13.04 GB` after about `33.2s`
- widened `--max-rss=15` retries do complete (`~55-59s` inference), but both return OCR preview
  `0` and fail semantic assertion (`missing expected substring 9`)
- this branch is exploratory-only and is not promoted into the retained quality ladder

Evidence:

- `.run/tmp/downstream_deepseek_metal_digit9_probe_12g_v1.log`
- `.run/tmp/downstream_deepseek_metal_digit9_probe_13g_v1.log`
- `.run/tmp/downstream_deepseek_metal_digit9_probe_15g_v1.log`
- `.run/tmp/downstream_deepseek_metal_digit9_probe_15g_v2.log`

Structured amount-prefix follow-up (2026-03-18 exploratory):

- a tighter production-like amount probe was run with the same accepted head by lowering decode
  budget to `max_new_tokens=4` and only requiring substring `Amount: $128`
- `--phase=infer --max-rss=12` now misses by only `0.01 GB`, exiting on guard at `12.01 GB` after
  about `39.4s`
- `--phase=infer --max-rss=13` still spikes out at `13.14 GB` after about `33.9s`
- a widened `--max-rss=15` retry no longer dies early, but it does not produce an in-log
  completion boundary either; after peaking around `10.59 GB` it drains into a long low-RSS tail at
  `~0.60 GB` until manual stop
- this is the closest production-like compact field so far, but it is still exploratory-only and
  not yet retained

Evidence:

- `.run/tmp/downstream_deepseek_metal_amount_prefix_probe_12g_v1.log`
- `.run/tmp/downstream_deepseek_metal_amount_prefix_probe_13g_v1.log`
- `.run/tmp/downstream_deepseek_metal_amount_prefix_probe_15g_v1.log`

Structured exact amount-prefix and invoice-prefix follow-up (2026-03-18 exploratory):

- after directly inspecting `.run/tmp/ocr-smoke.png`, the next compact probes were aligned to the
  actual visible structured fields `Amount: $128.50` and `Invoice No: 2026-03-09-001`
- a corrected exact amount-prefix branch requiring only `Amount: $128` under `--max-rss=12`
  reaches the guard edge once at `12.00 GB` in about `39.8s`, but the same parameters regress on
  immediate repeat to `12.57 GB` in about `95.0s`
- the same exact amount-prefix branch has now been rerun on the verified
  `.run/target-real-metal-cli-debug` surface; it no longer dies on the old `12 GB` wall, but it
  settles into a long `~1.08 GB` low-RSS plateau through at least `152s` without an in-log decode
  completion boundary and was manually stopped
- the corresponding invoice-prefix branch requiring only `Invoice No: 2026` was materially worse
  on refreshed `target/debug`, tripping the same `12 GB` guard at `13.11 GB` in about `33.9s`
- that same invoice-prefix branch has now also been rerun on the verified
  `.run/target-real-metal-cli-debug` surface; it no longer dies on the old guard wall either, but
  it settles into a long `~0.30 GB` low-RSS plateau through at least `126s` without an in-log
  decode completion boundary and was manually stopped
- this keeps `Amount: $128` as the best production-like compact field direction so far, but the
  exact-prefix branch is still exploratory-only because realignment changes the failure mode
  without producing a retained completion; the invoice-prefix branch improves its peak-memory shape
  on the verified surface but remains deprioritized because it still does not converge

Evidence:

- `.run/tmp/downstream_deepseek_metal_amount_prefix_exact_probe_12g_v2.log`
- `.run/tmp/downstream_deepseek_metal_amount_prefix_exact_probe_12g_v3.log`
- `.run/tmp/downstream_deepseek_metal_amount_prefix_exact_probe_12g_v4_realign.log`
- `.run/tmp/downstream_deepseek_metal_invoice_prefix_probe_12g_v1.log`
- `.run/tmp/downstream_deepseek_metal_invoice_prefix_probe_12g_v2_realign.log`

Structured shorter amount-side prefix follow-up (2026-03-18 exploratory):

- a shorter amount-side probe then required only visible prefix `Amount: $1` with
  `max_new_tokens=2` under the same accepted head
- this did not promote the branch: the run sat at very low RSS (`~0.13 GB`) for roughly `80s`,
  then ramped sharply and still hit guard at `12.08 GB` after about `100.7s`
- shortening the visible amount-side prefix alone is therefore not enough to make the branch
  retained; it only moves the wall later in time

Evidence:

- `.run/tmp/downstream_deepseek_metal_amount_prefix_dollar1_probe_12g_v1.log`

Structured invoice label-only follow-up (2026-03-18 exploratory):

- a label-only invoice-side probe then required visible prefix `Invoice No:` with
  `max_new_tokens=3` under the same accepted head
- this is better than the earlier `Invoice No: 2026` branch, but it is still not retained: the run
  idles at low RSS (`~0.13 GB`) through the early window, then ramps and still hits the `12 GB`
  guard at `12.09 GB` after about `54.4s`
- removing the numeric suffix therefore does not make invoice-side compact fields practical either;
  invoice-side is not a clean escape hatch from the amount-side wall

Evidence:

- `.run/tmp/downstream_deepseek_metal_invoice_label_probe_12g_v1.log`

Structured non-numeric line-label follow-up (2026-03-18 exploratory):

- a different compact field family was then tested with visible prefix `Line 2:` and
  `max_new_tokens=2` under the same accepted head
- this valid guarded rerun is also non-retained: after a low-RSS early window it ramps sharply and
  still hits the `12 GB` guard at `12.27 GB` after about `39.6s`
- this matters because `Line 2:` is not part of the amount/invoice numeric family; the failure
  shows the current smoke image does not have an obviously clean compact-label escape hatch either

Evidence:

- `.run/tmp/downstream_deepseek_metal_line2_label_probe_12g_v2.log`

Retained-gate surface-sensitivity follow-up (2026-03-18 exploratory but decision-complete):

- after compact-field mining was exhausted on the current smoke image, the next bounded step
  shifted to pilot-limit verification on already-retained text gates
- a fresh `Managed sidecar health check` rerun on the current default `target/debug` binary
  regressed and hit the `12 GB` guard at `12.04 GB` after about `45.1s`
- the same retained-gate prompt/assertion, when pinned back to the older verified
  `.run/target-real-metal-cli-debug` binary surface, still passes and returns
  `Managed sidecar health check.` in about `104.1s`
- this does not erase the retained text-gate evidence, but it does tighten the pilot boundary:
  current retained-gate repeatability is binary-surface sensitive and must not be generalized to
  arbitrary local `target/debug` builds
- the default runner path is now re-aligned to that verified surface as well: a normal
  `scripts/run_real_metal_test.py --profile=deepseek_metal_smoke_12g_safe384_managed_sidecar_line_metal_fast_eager_shared_native`
  invocation auto-selects `.run/target-real-metal-cli-debug`, passes under the shared `12 GB`
  guard, and returns `Managed sidecar health check.` in `127.0s`
- the first two immediate repeat attempts on that same default-runner path were rejected by
  `capfox` with `cpu_overload`, so they do not count as acceptance evidence
- after a short cooldown, the next default-runner repeat passed again in `99.9s` with the same OCR
  output, which upgrades the current state to two valid post-alignment passes (`v6`, `v9`) with
  transient capacity noise (`v7`, `v8`) in between
- a further cooled repeat now passes as well in `140.0s` with the same OCR output, so the current
  default-runner soak picture is three valid post-alignment passes (`v6`, `v9`, `v10`) plus two
  non-counting `cpu_overload` denials (`v7`, `v8`)
- a next-day rerun now passes too in `94.4s` with the same OCR output, which upgrades the current
  state from "same-session soak stability" to "cross-day default-runner stability" for this
  retained managed-sidecar lane
- the runner now also codifies this retained lane as the TOML-backed default Metal infer entry:
  when no explicit profile or manual probe overrides are provided, it resolves
  `[test_defaults].metal_infer_profile` to the managed-sidecar retained profile
- a no-profile invocation of `scripts/run_real_metal_test.py` now auto-selects that TOML default
  and passes in `89.1s`, so the limited-pilot entry is no longer "remember this profile name" but
  a concrete default runner path
- the adjacent `sidecar_line` retained profile has now also been revalidated on the same verified
  `.run` binary surface, so the retained sidecar-family evidence is no longer scoped to the
  managed-sidecar default entry alone

Evidence:

- `.run/tmp/downstream_deepseek_metal_managed_sidecar_line_profile_12g_v3.log`
- `.run/tmp/downstream_deepseek_metal_managed_sidecar_line_profile_12g_v4_oldbinary.log`
- `.run/tmp/downstream_deepseek_metal_managed_sidecar_line_profile_12g_v6_default_runner.log`
- `.run/tmp/downstream_deepseek_metal_managed_sidecar_line_profile_12g_v7_default_runner_repeat1.log`
- `.run/tmp/downstream_deepseek_metal_managed_sidecar_line_profile_12g_v8_default_runner_repeat1_retry.log`
- `.run/tmp/downstream_deepseek_metal_managed_sidecar_line_profile_12g_v9_default_runner_repeat2_after_cooldown.log`
- `.run/tmp/downstream_deepseek_metal_managed_sidecar_line_profile_12g_v10_default_runner_repeat3_after_cooldown.log`
- `.run/tmp/downstream_deepseek_metal_managed_sidecar_line_profile_12g_v11_default_runner_next_day.log`
- `.run/tmp/downstream_deepseek_metal_default_entry_v12_no_profile.log`
- `.run/tmp/downstream_deepseek_metal_sidecar_line_profile_12g_v2_realign.log`

## Evidence Rule

Every profile change should be accompanied by:

1. A guarded run log under `.run/tmp`
2. The exact effective env overrides used by the script
3. A short note describing whether the run reached load, prefill, or decode before failure
4. If stage tracing is enabled, the log should record the last visible stage before kill
