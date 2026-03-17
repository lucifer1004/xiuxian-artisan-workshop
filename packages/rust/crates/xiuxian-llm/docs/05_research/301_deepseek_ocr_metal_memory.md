# DeepSeek OCR Metal Memory Investigation

:PROPERTIES:
:ID: llm-deepseek-ocr-metal-memory
:PARENT: [[../index.md]]
:STATUS: ACTIVE
:END:

## Working Boundary

Current optimization work is restricted to vendored `deepseek-ocr`. We are explicitly not modifying `candle-core` while the remaining bottlenecks can still be explained at the model-execution layer.

## Confirmed Findings

### 1. Deferred Load Policy Scope Was Wrong

Lazy materialized MoE experts and CLIP transformer layers were falling back to the default `LowPrecisionLoadPolicy` after the initial model load guard exited.

This was confirmed because a guarded Metal run reported:

- `XIUXIAN_VISION_PRELOAD_LINEAR_WEIGHT_F32=0`
- but routed experts still showed `has_preloaded_weight_f32=true`

After fixing policy propagation, the same guarded path reported:

- `has_preloaded_weight_f32=false`
- explicit `weight_dtype=F16 -> target_dtype=F32` materialization in expert linear paths

Primary evidence:

- `.run/tmp/downstream_deepseek_metal_policy_fix_max1.log`

### 2. The Remaining Hot Path Is Model-Side MoE Execution

The validated post-fix kill point is inside routed MoE expert execution, not in `candle-core`, and not in the old policy-scope bug.

Observed path:

1. `decoder.forward.layer.started layer_idx=2`
2. `block.forward.moe.started layer_idx=2`
3. `block.forward.moe.expert.started layer_idx=2 expert_idx=0`
4. `block.forward.moe.linear.weight.materialize.started ... target_dtype=F32`
5. guard kill at `12.01 GB`

Primary evidence:

- `.run/tmp/downstream_deepseek_metal_policy_fix_max1.log`

### 3. The Previous Memory Model Was Incomplete

Before the policy fix, we were effectively measuring a path that still had preloaded F32 expert weights even when the profile said otherwise. That made later optimization conclusions unreliable.

## Rejected Experiment: Direct Weighted Scatter-Add

An experimental direct weighted scatter-add path was introduced in `transformer/block.rs` to reduce the `outputs -> cat -> scatter -> reshape/sum` chain.

Current status:

- the experiment regressed guarded Metal behavior and was reverted
- the accepted baseline is the pre-experiment accumulation path plus the deferred policy propagation fix
- subsequent guarded runs confirmed the reverted path remained materially healthier than the experiment

Primary evidence:

- `.run/tmp/downstream_deepseek_metal_scatter_clean.log`
- `.run/tmp/downstream_deepseek_metal_policy_fix_max1_scatter.log`
- `.run/tmp/downstream_deepseek_metal_revert_verify.log`

## Rejected Experiment: Routing-Scope / F32-Reuse Tightening

A later experiment tried two small-scope reductions inside `transformer/block.rs`:

- moving MoE routing intermediates into a narrower helper scope
- reusing existing F32 tensors instead of unconditional `to_dtype(F32)` calls

Current status:

- the change compiled and passed vendored unit tests
- the same guarded `deepseek_metal_guard_12g_safe448` profile regressed sharply and was reverted
- relative to the accepted baseline, the run was killed much earlier and never reached the deeper layer-5 progress seen in the revert verification run
- a post-revert probe confirmed the sharp `10.3s` regression was removed, but the current tree still exceeded the `12 GB` budget later in the same profile

Primary evidence:

- `.run/tmp/downstream_deepseek_metal_route_scope_max1.log`
- `.run/tmp/downstream_deepseek_metal_revert_verify.log`
- `.run/tmp/downstream_deepseek_metal_post_revert_probe.log`

## Practical Reading of the Current State

- The policy propagation fix is confirmed.
- The direct weighted scatter-add branch is rejected.
- The routing-scope / F32-reuse tightening branch is also rejected.
- The accepted baseline remains inside `deepseek-ocr` model execution, with no `candle-core` changes.
- The current tree no longer shows the sharp `10.3s` regression from the rejected branch, but it still has not reached a reproducible `status=0` under the `12 GB` guarded profile.
- The current routed expert path still needs additional lifetime reduction, especially around expert linear materialization, and every candidate change must be justified by a guarded run log.

## Current Smoke Baseline

A new conservative profile, `deepseek_metal_smoke_12g_safe384`, now exists to separate runtime stability from OCR quality assertions.

Observed behavior with the current accepted model baseline:

- with `base_size=384`, `image_size=384`, `crop_mode=false`, and `max_new_tokens=1`, Metal inference completed in about `39.5s`
- the same run stayed inside the `12 GB` guard, peaking around `11.20 GB`
- the OCR result was empty, so this run is valid as a memory-stability smoke but not as a quality-validating pass
- increasing the same profile to `max_new_tokens=8` exceeded the guard at about `12.11 GB`
- increasing the same profile to `max_new_tokens=4` also exceeded the guard at about `12.19 GB`
- after adding `allow_empty_output=true` to the smoke profile and rebuilding the real Metal test binary, the guarded profile produced a reproducible `status=0`
- the successful smoke run completed in about `43.6s` and stayed below the `12 GB` guard with a highest observed RSS around `10.72 GB`

Primary evidence:

- `.run/tmp/downstream_deepseek_metal_smoke_safe384.log`
- `.run/tmp/downstream_deepseek_metal_smoke_safe384_tokens8.log`
- `.run/tmp/downstream_deepseek_metal_smoke_safe384_tokens4.log`
- `.run/tmp/downstream_deepseek_metal_smoke_safe384_status0.log`

## New Boundary: The First Non-Empty Token Still Crosses `12 GB`

Follow-up work shifted from "runtime completes" to "runtime produces the smallest non-empty OCR
output." The smallest stable target tested so far is a single visible digit with
`max_new_tokens=1`.

### Config-Only Probe Results

- `384x384`, `crop_mode=false`, prompt `Return exactly one visible digit...`
  - exceeded the guard at about `12.12 GB`
  - evidence: `.run/tmp/downstream_deepseek_metal_smoke_safe384_digit1.log`
- `352x352`
  - failed earlier with `image embedding failed`
  - evidence: `.run/tmp/downstream_deepseek_metal_smoke_safe352_digit1_probe.log`
- `320x320`
  - exceeded the guard at about `12.04 GB`
  - evidence: `.run/tmp/downstream_deepseek_metal_smoke_safe320_digit1_probe.log`
- `256x256`
  - exceeded the guard at about `12.04 GB`
  - evidence: `.run/tmp/downstream_deepseek_metal_smoke_safe256_digit1_probe.log`
- `320x320` with `decode_use_cache=true`
  - still exceeded the guard at about `12.05 GB`
  - evidence: `.run/tmp/downstream_deepseek_metal_smoke_safe320_digit1_cache_probe.log`
- `320x320` with shorter prompts such as `One digit only.` and `Digit.`
  - still exceeded the guard at about `12.14 GB` and `12.03 GB`
  - evidence:
    - `.run/tmp/downstream_deepseek_metal_smoke_safe320_shortprompt_probe.log`
    - `.run/tmp/downstream_deepseek_metal_smoke_safe320_minprompt_probe.log`

### Practical Reading

- The remaining failure is no longer explained by image size alone.
- The remaining failure is also not explained by `use_cache`.
- The remaining failure survives even when the OCR prompt is reduced to a minimal text form.
- This makes the remaining overage look like a narrow but persistent prefill-side fixed cost, not
  a decode-loop or prompt-only problem.

## Confirmed Trace for the First-Token Investigation

With `XIUXIAN_VISION_STAGE_TRACE_STDERR=1`, the exploratory `320x320` digit probe reported:

- `input_tokens=50`
- `image_tokens=31`
- `use_cache=false`
- `max_new_tokens=1`

The trace clearly showed that the current hot path remained in no-cache prefill:

1. `ocr_engine.decode.build_prompt_tokens.completed input_tokens=50 image_tokens=31`
2. `decode.no_cache.prefill.started`
3. `decoder.forward.layer.started layer_idx=1`
4. `block.forward.moe.started layer_idx=1`
5. `decoder.forward.layer.started layer_idx=2`
6. `block.forward.moe.started layer_idx=2`

This matters because `max_new_tokens=1` never enters a second forward pass in
`generate_without_cache`. The remaining budget problem is therefore still a prefill-time MoE
problem, not a post-prefill token loop problem.

Primary evidence:

- `.run/tmp/downstream_deepseek_metal_smoke_safe320_digit1_trace.log`

## Rejected Experiment: No-Cache First-Token Fast Return

An experiment briefly changed vendored `deepseek-ocr` so that the `max_new_tokens=1`,
`use_cache=false` path returned immediately after first-token selection, in order to drop prefill
artifacts earlier.

Current status:

- the change compiled and passed vendored unit tests
- the guarded `320x320` digit probe still exceeded the `12 GB` budget
- the experiment was reverted and is not part of the accepted baseline

Primary evidence:

- `.run/tmp/downstream_deepseek_metal_smoke_safe320_digit1_post_drop.log`

## Accepted Tightening: On-Demand N-Gram Filter Cloning

A small but accepted helper change now avoids cloning a full filtered logits vector when no-repeat
ngram filtering does not actually ban any tokens.

Why it is kept:

- it preserves the existing deterministic sampling semantics
- it keeps the current tree simpler than the rejected visible-first steering branch
- it improved the current guarded single-digit baseline from the higher `12.1 GB+` range down to a
  latest recheck around `12.04 GB`

Primary evidence:

- `.run/tmp/downstream_deepseek_metal_safe384_digit1_native_inputs_native_attn_native_gate_inputs_post_visible_revert.log`

## Rejected Experiment: Visible-First Token Steering

The next experiment tried to skip a whitespace-only first token before final text normalization.
The goal was to convert the current empty-output failure into a minimal visible output without
raising the memory budget.

Current status:

- rejected and removed from the current tree
- repeated guarded runs stayed above the `12 GB` budget
- the steering path added complexity without producing a reproducible memory-stable non-empty probe
- the retained improvement is the lighter sampling-helper allocation behavior, not the visible-first
  selection path itself

Primary evidence:

- `.run/tmp/downstream_deepseek_metal_safe384_digit1_native_inputs_native_attn_native_gate_inputs_visible_first.log`

## New Experiment: Native-Dtype MoE Expert Compute

A new bounded experiment disabled forced F32 compute inside routed and shared MoE expert MLPs while
keeping the rest of the pipeline unchanged. The control surface is the test-only env/profile flag:

- `XIUXIAN_VISION_MOE_EXPERT_F32_COMPUTE=0`

This was implemented inside vendored `deepseek-ocr` only. It does not modify `candle-core`, and it
does not change the default path.

### Result

- debug guarded run with `base_size=320`, `image_size=320`, and the single-digit prompt
  - still exceeded the guard at about `12.02 GB`
  - evidence: `.run/tmp/downstream_deepseek_metal_safe320_digit1_native_moe.log`
- release guarded run with the same profile
  - exceeded the guard earlier, at about `12.17 GB`
  - evidence: `.run/tmp/downstream_deepseek_metal_safe320_digit1_native_moe_release.log`
- shortening the prompt further while staying on the same profile shape still did not get below the
  guard
  - evidence: `.run/tmp/downstream_deepseek_metal_safe320_native_moe_minprompt.log`

### Practical Reading

- Forced F32 MoE expert compute is part of the remaining pressure, but it is not the only blocker.
- Disabling F32 expert compute reduced the overage only slightly in debug.
- The release path remained worse than debug, so this toggle alone is not enough to define a
  passing non-empty smoke profile.
- The next step should therefore look beyond expert compute mode alone, while still staying inside
  vendored `deepseek-ocr`.

## New Experiment: Native-Dtype Language Inputs

A new bounded experiment disabled the prefill-time promotion from low precision to F32 for
language input embeddings. The control surface is:

- `XIUXIAN_VISION_PROMOTE_LANGUAGE_INPUT_F32=0`

This change is still scoped to vendored `deepseek-ocr`. It does not modify `candle-core`, and it
does not change the default path.

### Result

- guarded debug run with `base_size=384`, `image_size=384`, and the single-digit prompt
  - still exceeded the guard, but only at about `12.12 GB`
  - evidence: `.run/tmp/downstream_deepseek_metal_safe384_digit1_native_inputs.log`

### Practical Reading

- This toggle is materially more promising than the earlier `native_moe` experiment.
- The overage moved from "immediate failure" to a narrow late-stage miss, which means the language
  input F32 promotion was contributing real pressure.
- The improvement is still not enough to define a passing non-empty `12 GB` profile by itself.
- The remaining work should stay focused on prefill-time decoder activations or hidden-state dtype
  handling, not on MoE expert compute alone.

## Rejected Combination: Native Inputs Plus Native MoE Compute

The next bounded follow-up combined both of the active experimental toggles:

- `XIUXIAN_VISION_PROMOTE_LANGUAGE_INPUT_F32=0`
- `XIUXIAN_VISION_MOE_EXPERT_F32_COMPUTE=0`

### Result

- the same guarded `384x384` single-digit profile exceeded the guard even earlier, at about
  `12.02 GB`
- evidence: `.run/tmp/downstream_deepseek_metal_safe384_digit1_native_inputs_native_moe.log`

### Practical Reading

- The combined profile did not preserve the improvement seen from native language inputs alone.
- This means "turn off more F32" is not a safe heuristic by itself.
- The accepted next direction should not keep stacking expert-compute toggles. It should explain
  why the decoder-side prefill path still holds onto enough state to cross the last `12 GB`
  boundary.

## New Experiment: Native Inputs Plus Native Prefill Attention

The next bounded follow-up kept the native language-input experiment and disabled the prefill-side
attention F32 path:

- `XIUXIAN_VISION_PROMOTE_LANGUAGE_INPUT_F32=0`
- `XIUXIAN_VISION_PREFILL_ATTENTION_F32=0`

The decode-only `seq_len=1` F32 path was left unchanged. This experiment only targeted the
prefill-time attention path for `seq_len > 1`.

### Result

- the guarded `384x384` single-digit profile still exceeded the `12 GB` guard
- the overage was slightly smaller than the native-input-only profile, landing around `12.10 GB`
- evidence: `.run/tmp/downstream_deepseek_metal_safe384_digit1_native_inputs_native_attn.log`

### Practical Reading

- Prefill attention F32 is contributing pressure, but not enough to be the dominant remaining
  blocker by itself.
- The result is directionally better than native inputs alone, but only by a very small margin.
- The next experiment should not keep slicing attention in isolation. The stronger hypothesis now
  is that the remaining pressure sits in prefill-time hidden-state / residual lifetime across the
  decoder block, not in attention-only policy.

## New Code-Reading Boundary: MoE Routing and Combine Still Stage Large F32 State

Follow-up inspection of vendored `deepseek-ocr` narrowed the next likely hotspot further than
attention:

- in `transformer/block.rs`, `run_moe(...)` still converts `tokens` to `tokens_f32`
- gating weights are materialized as `gate_weight.to_dtype(F32)`
- expert outputs are accumulated as `Vec<Tensor>` in `F32`
- the scatter buffer `new_x` is allocated as `Tensor::zeros(..., DType::F32, ...)`
- top-k weights are kept in `F32` for the final combine path

This matters because it explains why:

- native expert compute alone was not enough
- native prefill attention was also not enough

Even after those reductions, routed-expert scheduling and combine still preserve a large F32
working set during prefill.

### Practical Reading

- The next bounded experiment should target `run_moe(...)`, especially routing and combine staging.
- The strongest next candidates are not more attention-only toggles.
- The next work should stay inside vendored `deepseek-ocr`, with no `candle-core` changes.

## Rejected Combination: Native Inputs Plus Native Prefill Attention Plus Native MoE Combine

The next bounded follow-up kept the two most promising toggles and then switched MoE
expert-output/scatter/combine working tensors away from `F32`:

- `XIUXIAN_VISION_PROMOTE_LANGUAGE_INPUT_F32=0`
- `XIUXIAN_VISION_PREFILL_ATTENTION_F32=0`
- `XIUXIAN_VISION_MOE_COMBINE_F32=0`

This change kept MoE routing decisions in `F32`, but moved expert-output staging, scatter, and the
top-k combine path to the hidden-state dtype.

### Result

- the guarded `384x384` single-digit profile regressed
- it exceeded the `12 GB` guard earlier, at about `12.23 GB`
- evidence:
  - `.run/tmp/downstream_deepseek_metal_safe384_digit1_native_inputs_native_attn_native_combine.log`

### Practical Reading

- The MoE combine/scatter working dtype is not the main remaining wall.
- Lowering combine precision did not preserve the gains from native inputs and native prefill
  attention.
- The next likely targets inside `run_moe(...)` are now narrower:
  - gating and `tokens_f32` materialization
  - expert-side intermediate tensors

## New Boundary: Native MoE Gating Inputs Clear the Memory Budget

The next bounded follow-up stayed inside `run_moe(...)`, but moved one level earlier than
combine/scatter. Instead of changing expert execution or MoE combine buffers, it kept routing
decisions in `F32` while switching the routing matmul inputs back to the hidden-state dtype:

- `XIUXIAN_VISION_PROMOTE_LANGUAGE_INPUT_F32=0`
- `XIUXIAN_VISION_PREFILL_ATTENTION_F32=0`
- `XIUXIAN_VISION_MOE_GATE_INPUT_F32=0`

This changed:

- routed token staging from unconditional `tokens_f32` materialization to native-dtype inputs
- gating-weight materialization from unconditional `gate_weight.to_dtype(F32)` to native-dtype
  inputs
- the routing logits, softmax, sort, and top-k path still remained in `F32`

### Result

- the guarded `384x384` single-digit profile no longer hit the `12 GB` kill threshold
- observed memory rose to about `10.78 GB` near the early warm-up peak
- the run completed inference and dropped back to about `6.09 GB` by the end of the test
- evidence:
  - `.run/tmp/downstream_deepseek_metal_safe384_digit1_native_inputs_native_attn_native_gate_inputs.log`

### Practical Reading

- The remaining blocker is no longer memory budget for this probe shape.
- The failure moved from "killed by guard" to "inference completed but OCR output was empty."
- This is the clearest sign so far that the next work should pivot from memory containment toward
  first-token / output-quality behavior under the same budget.

## Empty-Output Diagnosis: The First Token Is Whitespace, Not EOS

To avoid perturbing the guarded memory shape with full stage tracing, a one-line empty-output
diagnostic was added behind `XIUXIAN_VISION_TRACE_EMPTY_OUTPUT=1`.

Using the current best single-digit profile:

- `promote_language_input_f32=false`
- `prefill_attention_f32=false`
- `moe_gate_input_f32=false`

the guarded run stayed under the `12 GB` budget and completed inference, but the empty-output
diagnostic showed:

- `response_tokens=1`
- `token_preview=[6776]`
- `decoded_preview="  \\n\\n"`
- `normalized_chars=0`

Evidence:

- `.run/tmp/downstream_deepseek_metal_safe384_digit1_native_inputs_native_attn_native_gate_inputs_emptytrace.log`

### Practical Reading

- The profile is no longer failing because of EOS termination at zero tokens.
- It is generating a first token, but that token decodes to whitespace and is removed by
  `normalize_text(...).trim()`.
- The next useful target is first-token steering under the same budget, not additional memory work.

## Prompt-Only Follow-Ups: Easy to Regress the Prefill Budget

Two prompt-only probes were tried on top of the same memory-stable profile:

- a longer anchored prompt ending in `Digit:`
- a very short prompt `<image>\n<|grounding|>Digit:`

Both regressed the guarded run back above the `12 GB` limit:

- the longer prompt climbed back to about `12.10 GB`
- the short prompt also regressed and was killed at about `12.03 GB`

Evidence:

- `.run/tmp/downstream_deepseek_metal_safe384_digit1_native_inputs_native_attn_native_gate_inputs_promptprobe.log`
- `.run/tmp/downstream_deepseek_metal_safe384_digit1_native_inputs_native_attn_native_gate_inputs_shortprompt.log`

### Practical Reading

- Prompt shaping is not "free" on this path; even small prompt changes can move the prefill budget.
- The next quality-side probes should stay narrowly bounded and be compared against the current
  stable memory profile, not assumed safe just because they do not change model code.
