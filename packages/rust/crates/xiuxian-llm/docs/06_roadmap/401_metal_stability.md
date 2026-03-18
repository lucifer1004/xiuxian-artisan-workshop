# Metal Stability Roadmap

:PROPERTIES:
:ID: llm-metal-stability-roadmap
:PARENT: [[../index.md]]
:STATUS: ACTIVE
:END:

## Immediate Goal

Reach a guarded DeepSeek OCR Metal inference path that completes within a `12 GB` RSS budget before attempting broader optimization.

Current status: the conservative smoke profile `deepseek_metal_smoke_12g_safe384` now satisfies that runtime-stability goal. The next work should keep that baseline green while improving output quality under the same budget.

The current quality-side target is narrower:

- keep `deepseek_metal_smoke_12g_safe384` green
- make the smallest non-empty OCR output fit under the same `12 GB` guard

## Current Sequencing

1. Preserve upstream-parity configuration and loader behavior.
2. Fix model-side policy or execution bugs inside vendored `deepseek-ocr`.
3. Re-run the guarded non-empty probe after each material model change.
4. Only after the smallest non-empty output is green under the same budget should broader quality or latency tuning resume.

## Current Candidate Work Items

### A. Keep the reverted MoE accumulation baseline

The direct weighted scatter-add path in `transformer/block.rs` has been rejected. The next work starts from the reverted baseline that kept deeper guarded Metal progress stable.

### B. Reduce routed expert temporary pressure

If the accumulation path is not enough, the next work must target routed expert `gate/up/down` temporary lifetimes without moving into `candle-core`.

The recent routing-scope / F32-reuse tightening attempt regressed the guarded `12 GB` path and has been rejected, so the next attempt must not repeat that shape blindly.

Recent config-only probes also showed that smaller images, cache mode changes, and shorter prompts do not remove the remaining overage. That narrows the next implementation step back to model-side prefill work.

### C. Separate routed and shared expert accounting

Shared experts should be measured independently so that future memory conclusions do not conflate routed expert pressure with shared expert materialization.

### D. Confirm prefill-only MoE pressure before any new code motion

The latest `max_new_tokens=1` stage trace showed:

- `input_tokens=50`
- `image_tokens=31`
- no second no-cache decode forward
- active work still inside routed MoE experts for decoder layers `1` and `2`

That means the current overage is still a prefill-time MoE problem. Future changes should be justified against that boundary, not against token-loop hypotheses.

### E. Reject first-token fast-return as a standalone fix

An early-return experiment for the `max_new_tokens=1` no-cache path did not recover the missing headroom and has been reverted. The next work should not revisit that idea unless new evidence shows the retained tensors were the dominant cost.

### F. Treat native-dtype MoE expert compute as insufficient on its own

The test-only `moe_expert_f32_compute=false` experiment is worth keeping for research, but it did
not clear the `12 GB` guard by itself:

- debug still landed at about `12.02 GB`
- release was worse, around `12.17 GB`

This means the next bounded change should not be "more of the same" inside expert compute alone.
The next useful target should explain the remaining fixed overage that survives after expert
compute is already moved off F32.

### G. Treat native language-input dtype as the current lead, but not the finish line

The test-only `promote_language_input_f32=false` experiment improved the `384x384` single-digit
probe substantially, but it still landed just above the guard:

- debug reached about `12.12 GB` before kill

That makes language-input dtype the strongest currently validated lead. It does not yet justify a
new default profile, but it does justify further investigation around prefill-time hidden-state
promotion and decoder activation lifetime.

### H. Reject the "stack both toggles" shortcut

Combining:

- `promote_language_input_f32=false`
- `moe_expert_f32_compute=false`

did not recover the remaining headroom. The combined `384x384` single-digit probe failed earlier
than the native-input-only profile.

This means the next step should not be another boolean-stack experiment. The next useful target is
the decoder-side prefill path that still turns a near-pass into a `12 GB` miss.

### I. Treat prefill attention F32 as a minor contributor, not the main wall

Disabling the prefill-only low-precision attention F32 path on top of native language inputs
improved the guarded `384x384` single-digit probe only slightly. The run still landed at about
`12.10 GB`.

That means the next useful target should move one level up:

- decoder block hidden-state lifetime
- residual / norm staging between attention and MoE

The next step should therefore avoid another attention-only toggle and instead focus on the
prefill block boundary itself.

### J. Shift the next bounded experiment into `run_moe(...)` routing/combine

Current code reading shows that prefill-time MoE still keeps a large F32 working set even after the
recent input and attention reductions:

- routed token staging is converted to `tokens_f32`
- gating weights and scores stay in `F32`
- expert outputs are gathered in `Vec<Tensor>` as `F32`
- scatter/combine buffers also stay in `F32`

That makes `run_moe(...)` routing and combine the next best bounded target. The next change should
therefore focus there, not on another block-attention experiment and not on `candle-core`.

### K. Narrow the next `run_moe(...)` target again: not combine, but gating or expert intermediates

The follow-up `moe_combine_f32=false` experiment regressed the guarded `384x384` single-digit
probe. That means the combine/scatter working dtype is not the right next lever by itself.

The next bounded target should therefore shift again, still inside `run_moe(...)`, but away from
combine:

- gating path and `tokens_f32` materialization
- expert-side intermediate tensors

### L. Treat native MoE gating inputs as the memory-budget breakthrough for the single-digit probe

The follow-up routing-input experiment changed only the MoE gating matmul inputs:

- `promote_language_input_f32=false`
- `prefill_attention_f32=false`
- `moe_gate_input_f32=false`

This kept the routing decision path in `F32`, but stopped materializing `tokens_f32` and
`gate_weight.to_dtype(F32)` up front.

Result:

- the guarded `384x384` single-digit probe completed without tripping the `12 GB` guard
- the run then failed because OCR output was empty, not because memory was exhausted

That shifts the next immediate goal:

- keep this memory behavior reproducible
- focus the next investigation on first-token / output-quality behavior under the same `12 GB`
  budget
- avoid reopening broader memory work unless new evidence shows the budget regresses

### M. Treat first-token whitespace as the new blocker, not EOS or OOM

With the lightweight empty-output diagnostic enabled, the current best guarded profile showed:

- `response_tokens=1`
- `token_preview=[6776]`
- decoded preview was whitespace-only

This means the single-digit probe is now failing because the first generated token normalizes to an
empty string, not because generation terminates at EOS and not because the run is being killed by
memory.

### N. Treat prompt-only steering as budget-sensitive

Two prompt-only attempts to bias the first token away from whitespace both regressed the guarded
run back above `12 GB`.

That means future quality-side work should follow these constraints:

- keep prompt probes minimal and measured against the current stable memory profile
- do not assume a prompt-only change is free just because it avoids model-code edits
- prefer targeted first-token investigation over larger prompt rewrites

### O. Reject visible-first token steering and keep only the helper-level memory win

The follow-up visible-first branch tried to skip a whitespace-only first token before final text
normalization. That path did not justify itself:

- repeated guarded runs stayed above the `12 GB` budget
- the branch added complexity inside vendored `deepseek-ocr`
- the branch has been removed from the current tree

What remains useful from the same research loop is narrower:

- the deterministic sampling helper now avoids an unnecessary full filtered-logits clone when
  n-gram blocking does not actually apply
- this brought the current best single-digit guarded recheck down to roughly `12.04 GB`, but still
  not below budget

That means the next step should not revisit visible-first steering immediately. The next bounded
work should stay on the existing best profile shape and target the remaining fixed prefill cost
that still keeps the single-digit probe a few tens of megabytes above budget.

### P. Prioritize a Metal MoE fast backend before any Candle-level work

Fresh comparison against `mistralrs-core` tightened the architectural direction:

- `mistralrs-core` already uses `mmap`-backed loading, but vendored `deepseek-ocr` does too
- the larger reusable difference is `mistralrs-core`'s dedicated Metal MoE fast path
- that fast path avoids the current `run_moe(...)` loop + `Vec<Tensor>` + scatter/combine shape

This changes the order of operations:

1. keep `candle-core` out of scope
2. treat `run_moe(...)` backend shape as the next major optimization target
3. only revisit load-time device placement after the Metal MoE backend question is exhausted

The next implementation spike should therefore ask:

`Can vendored deepseek-ocr adopt a Metal-oriented gather-style MoE backend similar to mistralrs-core before we consider lower-level runtime changes?`

### Q. Keep the new MoE backend seam stable and isolated

The first structural landing for that direction is now present:

- `MoeWeights` has an explicit backend-aware layout
- the default backend remains `slow`
- a `metal_fast` skeleton backend exists and currently falls back to the slow helper
- the `metal_fast` branch now carries a packed-expert container for eager float experts

This is not the optimization itself. It is the containment boundary for future optimization work.

That changes how the next experiments should be judged:

- fast-path experiments should happen behind the `metal_fast` backend branch
- the existing slow path should stay available as the control path
- guarded regressions should be measured as backend-branch regressions, not as general model
  instability

The next step after this landing is therefore narrower than before:

1. keep the new backend seam compile-stable
2. move the first real gather-style or fused-weight forward experiment into the `metal_fast` branch
3. do not change the default backend until the branch produces a guarded Metal win

The immediate implementation target is now narrower than "build a fast backend":

- use the packed expert carrier already present in the `metal_fast` branch
- avoid further selector or layout churn
- keep the fallback-to-slow behavior until the first packed forward path is measurable

### R. Treat the first packed routed-expert forward as a structural landing, not a budget win

The `metal_fast` branch now does more than select a backend name:

- eager float experts can be packed into `gate/up/down` expert tensors
- routed experts can execute through that packed carrier instead of resolving per-expert
  `DenseMlpWeights`

That branch is now real enough to probe, and the first guarded result is clear:

- the existing stable lazy-expert profile does not exercise the packed path
- a manual eager-expert probe does exercise it
- that eager probe still lands above the budget, around `12.13 GB`

So the next step is no longer "make the branch real." The branch is real.

The next step is:

1. keep the packed routed-expert path as the experimental `metal_fast` branch
2. stop treating packed expert tensors alone as the likely breakthrough
3. target the remaining slow-path shape inside the branch:
   routed loop retention, `Vec<Tensor>` accumulation, and scatter/combine staging

### S. Treat token-major packed routed experts as informative, but still above budget

The next bounded experiment has now landed too:

- the `metal_fast` branch no longer needs the explicit routed-expert `Vec<Tensor>` accumulation
- it no longer needs the old `scatter_set` combine path for routed experts
- it can execute a token-major packed routed-expert path instead

That branch is still not under budget. The first guarded eager-expert probe reached roughly
`12.23 GB`, slightly worse than the earlier packed-routed branch.

That changes the next diagnosis:

- simply removing loop retention and scatter/combine is not sufficient
- the dominant remaining pressure is likely earlier than combine:
  eager packed-weight construction and/or the per-slot packed gather matmul shape

The next useful step should therefore avoid spending another round on output-accumulation cleanup.
It should inspect the packed-weight load shape and the slot-wise gather/matmul working set.

### T. Treat grouped-by-expert packed routing as the first measurable win inside `metal_fast`

The next bounded follow-up has now landed as well:

- the `metal_fast` branch no longer has to select packed expert weights once per token/slot pair
- it can group token positions by expert and run each packed expert once per grouped slice
- the eager guarded Metal probe is still above budget, but it improves from the earlier
  `~12.23 GB` result to roughly `12.08 GB`

This is not yet a budget success, but it is the first clear sign that the fast branch is moving in
the right direction.

That changes the next step again:

1. keep the grouped-by-expert packed routed path as the current `metal_fast` head
2. stop treating slot-wise packed gather/matmul as a solved question, because it still leaves a
   small but real overage
3. focus the next investigation on what remains after grouping:
   eager packed-weight retention and the residual routed/shared-expert working set

Evidence is recorded in:

- `.run/tmp/downstream_deepseek_metal_metal_fast_grouped_probe_fresh.log`

### U. Reject the non-contiguous packed-weight view experiment

One more bounded follow-up has now been ruled out:

- the grouped-by-expert `metal_fast` path was temporarily changed to keep packed expert weights as
  views instead of forcing `contiguous()` materialization
- the guarded eager probe regressed to roughly `12.20 GB` in about `13.5s`
- the experiment was reverted after verification

This removes another tempting shortcut from the search space:

1. do not assume that avoiding every explicit materialization is automatically better on Metal
2. keep the current grouped-by-expert path as the active `metal_fast` head
3. continue investigating the remaining routed/shared-expert working set rather than re-opening the
   packed-weight view question

Evidence is recorded in:

- `.run/tmp/downstream_deepseek_metal_metal_fast_grouped_viewprobe.log`

### V. Reject runtime pack as a replacement for load-time prepack

Another bounded follow-up has now been ruled out:

- the `metal_fast` branch was temporarily changed so eager probes could skip load-time expert-pack
  construction and build the pack inside the forward path instead
- the guarded eager probe regressed to roughly `12.10 GB` in about `6.7s`
- the experiment was reverted after verification

This keeps the next branch choice clear:

1. preserve load-time prepack as the current `metal_fast` head
2. do not revisit runtime pack unless a new design also changes the forward working set
3. continue investigating the grouped routed/shared-expert execution shape, not pack timing alone

Evidence is recorded in:

- `.run/tmp/downstream_deepseek_metal_metal_fast_runtimepack_probe.log`

### W. Treat slow-path routing staging removal as the current `metal_fast` head

The next bounded follow-up has now produced the best eager `metal_fast` result so far:

- `metal_fast` was refactored so it no longer eagerly builds slow-path-only routing tensors when
  the fast branch is selected
- this removes fixed staging such as `sorted_tokens` and `idxs`
- the guarded eager probe improved again to roughly `12.03 GB`

This changes the remaining problem shape:

1. keep this staging-trimmed branch as the new `metal_fast` head
2. stop revisiting pack timing and packed-weight view shape
3. focus the next bounded experiment on the residual fast-path working set:
   slot-level combine and shared-expert execution

Evidence is recorded in:

- `.run/tmp/downstream_deepseek_metal_metal_fast_minimal_staging_probe.log`

### X. Reject `scatter_add_set` as the slot-level combine fix

The next bounded follow-up has now been ruled out too:

- the fast branch temporarily replaced `slot_out -> weighted -> combined.add(...)` with direct
  `scatter_add_set` accumulation into the combined tensor
- the guarded eager probe regressed to roughly `12.37 GB` in about `8.1s`
- the branch was reverted after verification

This narrows the next useful step again:

1. keep the minimal-staging branch as the current `metal_fast` head
2. stop spending cycles on combine-shape rewrites for now
3. focus the next bounded investigation on the remaining expert-side working set, especially
   `shared_experts`

Evidence is recorded in:

- `.run/tmp/downstream_deepseek_metal_metal_fast_scatter_add_probe.log`

### Y. Treat shared-expert execution as the next bounded budget target

The next paired guarded probes produced the first useful attribution signal for the eager
`metal_fast` branch:

- a TOML-backed explicit eager baseline reached roughly `14.29 GB`
- a paired diagnostic run with `skip_shared_experts = true` reached roughly `13.43 GB`
- both runs were executed through named profiles so the backend selector and eager-MoE policy are
  visible in the runner log

This is not a shippable configuration, but it is a strong directional result:

1. `shared_experts` are now a confirmed material part of the remaining over-budget working set
2. the next bounded optimization should target shared-expert execution before revisiting routed
   combine shape again
3. a dedicated `XIUXIAN_VISION_SHARED_EXPERT_F32_COMPUTE` toggle now exists so shared-expert
   dtype can be tested without changing the routed-expert path

Evidence is recorded in:

- `.run/tmp/downstream_deepseek_metal_metal_fast_profile_baseline.log`
- `.run/tmp/downstream_deepseek_metal_metal_fast_profile_skip_shared.log`

### Z. Keep shared-expert native compute, reject native residual add

The next two bounded shared-side experiments have now been separated cleanly:

- lowering shared-expert compute to native dtype improved the eager `metal_fast` probe to a best
  observed result of roughly `12.12 GB`
- a follow-up repeat still landed around `12.33 GB`, so the branch is close but not yet stably
  under budget
- evidence is recorded in
  `.run/tmp/downstream_deepseek_metal_metal_fast_profile_shared_native_post_revert.log`
  and `.run/tmp/downstream_deepseek_metal_metal_fast_profile_shared_native_repeat.log`
- pushing further by also forcing the final shared residual add to stay native regressed to
  roughly `12.83 GB`
- evidence is recorded in
  `.run/tmp/downstream_deepseek_metal_metal_fast_profile_shared_native_residual_native.log`

This keeps the next step narrow:

1. retain `shared_expert_f32_compute = false` as the current best shared-side head
2. do not keep the native residual-add experiment in the tree
3. continue searching the remaining `~0.12-0.33 GB` gap elsewhere in the shared/routed working set

### AA. Reject token-major shared-input reuse as a follow-up to shared-native compute

One more bounded shared-side follow-up has now been tested and rejected:

- the shared-expert path was temporarily rewritten to reuse the token-major routed input instead
  of the original 3D hidden-state tensor
- after rebuilding an isolated real-Metal binary and rerunning the same guarded profile, the run
  still landed around `12.15 GB`
- evidence is recorded in
  `.run/tmp/downstream_deepseek_metal_metal_fast_profile_shared_native_token_reuse.log`

This means:

1. the accepted head stays `shared_expert_f32_compute = false`
2. shared-input code motion alone is not enough to recover the remaining headroom
3. the next bounded work should move past shared-input reuse and continue tracing the remaining
   routed/shared working-set variance

### AB. Reject lazy shared-expert loading as a load-spike-only improvement

Another bounded shared-side experiment has now been tested and rejected:

- the code briefly supported `lazy_shared_experts`, while keeping routed experts eager and the
  accepted `shared_expert_f32_compute = false` head intact
- this improved the load-only spike from roughly `12.72 GB` to roughly `12.30 GB`
- evidence is recorded in
  `.run/tmp/downstream_deepseek_metal_metal_fast_profile_shared_lazy_load_only.log`
- but the real infer probe regressed to roughly `12.58 GB`
- evidence is recorded in
  `.run/tmp/downstream_deepseek_metal_metal_fast_profile_shared_lazy_infer.log`
- the `lazy_shared_experts` branch was therefore reverted instead of being kept as another dormant
  policy toggle

### AC. Reject the packed-weight view experiment on the accepted shared-native head

One more bounded fast-path follow-up has now been tested and rejected:

- the packed routed-expert helpers were temporarily changed to keep selected packed weights as
  non-contiguous views instead of eagerly materializing them
- after rebuilding an isolated real-Metal binary and rerunning the accepted shared-native guarded
  profile, the result regressed to roughly `12.35 GB`
- evidence is recorded in
  `.run/tmp/downstream_deepseek_metal_metal_fast_profile_shared_native_weight_view.log`

This keeps the next step focused:

1. retain `shared_expert_f32_compute = false` as the current best accepted head
2. do not keep the packed-weight view change in the tree
3. continue looking for the remaining `~0.12-0.33 GB` inside fast-path working-set variance, not
   packed-weight view shape

### AD. Reject the per-expert 2D carrier rewrite for metal_fast

One more bounded structural follow-up has now been tested and rejected:

- the `metal_fast` carrier was temporarily rewritten to keep load-time contiguous per-expert 2D
  weights instead of the stacked packed tensors
- after rebuilding the isolated real-Metal binary and rerunning the accepted shared-native
  guarded profile, the first run landed around `12.20 GB`
- a repeat regressed much harder to roughly `13.14 GB`
- evidence is recorded in
  `.run/tmp/downstream_deepseek_metal_metal_fast_profile_shared_native_expert2d.log`
  and `.run/tmp/downstream_deepseek_metal_metal_fast_profile_shared_native_expert2d_repeat.log`

Across the rejected shared-side follow-ups, the roadmap still says:

1. retain the stacked packed carrier as the accepted head
2. do not keep the per-expert 2D carrier rewrite in the tree
3. continue investigating working-set variance inside the existing `metal_fast` carrier instead of
   replacing the carrier itself

### AE. Reject re-stacking native routed-expert compute on the accepted head

Another bounded boolean-stack follow-up has now been rechecked against the current architecture:

- the accepted shared-native `metal_fast` profile was rerun with `moe_expert_f32_compute = false`
  layered back on top
- the guarded result landed around `12.22 GB`
- evidence is recorded in
  `.run/tmp/downstream_deepseek_metal_metal_fast_profile_shared_native_native_moe.log`
- because this is still worse than the current best `~12.12 GB`, the transient profile was
  removed after verification

This keeps the roadmap honest:

1. the accepted head is still `metal_fast + shared_expert_f32_compute = false`
2. the routed-expert native-compute toggle is not a winning follow-up in the current architecture
3. the next step should stay on working-set variance inside the accepted head, not reopen this
   toggle stack

This keeps the roadmap disciplined:

1. do not retain load-only improvements that lose on guarded infer
2. keep the accepted head at eager shared weights plus `shared_expert_f32_compute = false`
3. continue hunting the remaining variance inside the retained eager path, not by reviving
   `lazy_shared_experts`

### AF. Use PTY-backed guarded tracing as the accepted-head diagnostic baseline

The latest accepted-head work produced a real observability landing:

- guarded Metal runs can now carry stage traces through the PTY-backed harness path
- `DeepseekOcrModel::load(...)` now emits early load-entry stages before the heavier load work
- the accepted head can now be attributed even when it dies very early

What the new evidence says:

- the accepted head still dies during load
- both guarded traces now cross:
  - `deepseek.load.weights_mmap.completed`
  - `deepseek.load.deferred_moe_source.completed`
  - `deepseek.load.language.started`
- the finer widened `load` trace now reaches MoE expert linear materialization itself
- the latest representative `load` probe dies during
  `model.layers.5.mlp.experts.59.up_proj.weight`
- the guarded `infer` trace reaches `deepseek.language.transformer_layer.start` for `layer_idx=5`
  after completing layer `4`

Evidence:

- `.run/tmp/downstream_deepseek_metal_metal_fast_profile_shared_native_trace_pty_load_15g_v6.log`
- `.run/tmp/downstream_deepseek_metal_metal_fast_profile_shared_native_trace_pty_13g_v3.log`

That changes the next bounded step:

1. keep the accepted head unchanged
2. keep using PTY-backed guarded tracing for accepted-head attribution
3. investigate the working set inside `LinearWeights::load` for MoE expert projections before
   reopening later MoE/runtime toggles

### AG. Keep the expert-view materialization trim and move the wall past contiguous load

The next bounded implementation spike is now validated and retained:

- eager routed-expert `LinearWeights::load` no longer forces contiguous materialization on the
  accepted `metal_fast` path
- the same routed-expert projections are now forced contiguous only when the `metal_fast` expert
  pack is built
- the pack path now emits explicit neutral stage boundaries so the next wall is attributable even
  after the trim:
  - `deepseek.language.transformer_layer.mlp.moe.pack.start`
  - `...pack.gate.completed`
  - `...pack.up.completed`
  - `...pack.down.completed`
  - `...pack.completed`

What the new evidence says:

- the widened `load` trace no longer dies inside `LinearWeights::load`
- the widened `load` trace now completes all language layers, reaches
  `deepseek.language.weights_ready`, and exits successfully under the `15 GB` guard
- the guarded `infer` trace still misses the `13 GB` budget, but the failure moved materially:
  - the older accepted-head baseline died in about `1.2s` at about `13.29 GB`
  - the new accepted-head infer trace dies in about `18.9s` at about `13.04 GB`
- the new infer wall is no longer best described as `get/contiguous` for layer `4`
- the current infer-side wall now appears during eager routed-expert fetch on layer `5`, before
  the layer `5` pack boundary is reached

Evidence:

- `.run/tmp/downstream_deepseek_metal_metal_fast_profile_shared_native_trace_pty_load_15g_v7.log`
- `.run/tmp/downstream_deepseek_metal_metal_fast_profile_shared_native_trace_pty_13g_v4.log`
- `.run/tmp/downstream_deepseek_metal_metal_fast_profile_shared_native_trace_pty_13g_v3.log`

This changes the next bounded step again:

1. keep the materialization trim in the accepted head
2. stop treating contiguous expert-weight materialization as the active wall
3. investigate eager routed-expert `weight.get(...)` residency and fetch ordering for layer `5`
   and later, not mmap, not pack tracing, and not `candle-core`

### AH. Reject the direct projection-pack rewrite for eager routed experts

The next bounded follow-up was to move eager routed-expert packing even earlier:

- load gate/up/down one projection at a time
- write each projection directly into a preallocated packed carrier
- avoid holding a transient `DenseMlpWeights` struct for every eager routed expert during widened
  `load`

That branch did not win and should stay out of the tree:

- the widened `load` regression reappeared under the same accepted-head profile
- the new `15 GB` guarded probe died at about `15.05 GB`
- the latest visible wall moved into layer `6` eager routed-expert `up_proj/down_proj`
  `weight.get(...)`
- this is strictly worse than the retained `v7` widened `load` result, which reached
  `deepseek.language.weights_ready` and exited successfully

Evidence:

- `.run/tmp/downstream_deepseek_metal_metal_fast_profile_shared_native_trace_pty_load_15g_v8.log`
- `.run/tmp/downstream_deepseek_metal_metal_fast_profile_shared_native_trace_pty_load_15g_v7.log`

This keeps the next bounded step narrow:

1. keep the retained expert-materialization trim exactly as-is
2. do not reopen the direct projection-pack rewrite
3. continue attribution on eager routed-expert `weight.get(...)` residency and fetch ordering
   from the stable accepted head

### AI. Re-anchor the 13 GB target against pure load, not only infer

The next diagnostic comparison is now in place:

- the accepted-head `phase=load --max-rss=13` run was captured with the same PTY-backed trace path
- pure load does not pass the `13 GB` guard
- the pure-load wall appears earlier than the current infer wall

What the evidence says:

- the new pure-load `13 GB` run dies in layer `4` while loading `shared_experts`
- the retained accepted-head `13 GB infer` run still dies later, in layer `5` routed-expert fetch
- therefore the remaining `13 GB` miss is not just infer-only overhead layered on top of a stable
  load path
- shared-expert load residency is still part of the active target-budget problem, even though the
  widened `15 GB` load run reaches `weights_ready`

Evidence:

- `.run/tmp/downstream_deepseek_metal_metal_fast_profile_shared_native_trace_pty_load_13g_v1.log`
- `.run/tmp/downstream_deepseek_metal_metal_fast_profile_shared_native_trace_pty_13g_v4.log`

This shifts the next bounded step slightly:

1. keep the retained accepted head unchanged
2. stop treating the remaining `13 GB` miss as routed-expert-only
3. re-open shared-expert load working-set attribution alongside eager routed-expert fetch order

Shared-expert attribution was then tightened one step further:

- the existing `skip_shared_experts` flag was aligned so it now short-circuits both forward use and
  load-time `shared_experts` materialization
- with that alignment in place, the `phase=load --max-rss=13` diagnostic still dies, but no longer
  in `shared_experts`; the wall moves to layer `4` routed-expert eager loads around expert `45`
- the matching `phase=infer --max-rss=13` diagnostic also still dies in layer `4`, around routed
  expert `38`
- this narrows the next optimization target again: shared-expert residency matters, but the active
  `13 GB` wall that remains after removing it is routed-expert eager fetch/materialization inside
  layer `4`

Evidence:

- `.run/tmp/downstream_deepseek_metal_metal_fast_profile_skip_shared_trace_pty_load_13g_v2.log`
- `.run/tmp/downstream_deepseek_metal_metal_fast_profile_skip_shared_trace_pty_13g_v1.log`

One more attribution check narrowed it further:

- with the same `skip_shared` shape, but only `lazy_moe_experts = true`, the manual
  `phase=load --max-rss=13` repro passed cleanly and reached `weights_ready`
- the observed load RSS stayed around `6.62 GB`, far below the target budget
- a matching no-trace `phase=infer --max-rss=13` repro was observed for `86s` without crossing the
  guard; observed RSS stayed at or below about `7.57 GB`
- this does not automatically promote lazy routed experts into the canonical head, but it does
  make the attribution decision-complete: the active `13 GB` wall is dominated by eager routed-
  expert residency

Evidence:

- `.run/tmp/downstream_deepseek_metal_manual_lazy_skip_shared_trace_pty_load_13g_v2.log`
- `.run/tmp/downstream_deepseek_metal_manual_lazy_skip_shared_infer_13g_notrace_v1.log`

### AK. Retain internal `metal_fast` routed-expert deferral and move the wall into forward execution

The retained profile stays the same at the surface:

- `deepseek_metal_smoke_12g_safe384_digit1_native_inputs_native_attn_native_gate_inputs_metal_fast_eager_shared_native`

But the retained `metal_fast` implementation now changes routed-expert load strategy internally:

- when a deferred source exists and no snapshot is present, routed experts now stay deferred under
  `metal_fast` even though the external `lazy_moe_experts` flag remains off
- shared experts remain eager
- no new TOML profile or env toggle was introduced

Observed impact:

- widened `phase=load --max-rss=15` now passes cleanly and reaches `weights_ready` in about `14.1s`
- guarded `phase=infer --max-rss=13` still fails, but only after model load completes; it reaches
  `weights_ready`, enters `block.forward.moe`, and then dies at about `13.02 GB` after about `40.4s`
- the active wall is therefore no longer load-time routed-expert residency; it is now deferred
  routed-expert forward materialization

Evidence:

- `.run/tmp/downstream_deepseek_metal_metal_fast_profile_shared_native_trace_pty_load_15g_v9.log`
- `.run/tmp/downstream_deepseek_metal_metal_fast_profile_shared_native_trace_pty_13g_v5.log`

### AK. Keep deferred routed-expert forward weights view-backed until projection use

The next retained landing stays inside vendored `deepseek-ocr` and does not change the canonical
profile surface:

- deferred routed experts still resolve on demand under `metal_fast`
- but `DeferredDenseMlpWeights::materialize(...)` now preserves view-backed `gate/up/down`
  weights instead of forcing all three projections contiguous immediately
- each projection is allowed to become contiguous only when the linear path actually consumes it

Observed impact:

- widened `phase=load --max-rss=15` still passes, now with observed RSS around `10.36 GB`
- guarded no-trace `phase=infer --max-rss=13` now also stays within budget, with observed RSS
  around `9.19 GB`
- the run no longer fails on memory; it reaches the end of inference and then fails because OCR
  output is empty

Evidence:

- `.run/tmp/downstream_deepseek_metal_metal_fast_profile_shared_native_load_15g_ephemeral_v2.log`
- `.run/tmp/downstream_deepseek_metal_metal_fast_profile_shared_native_infer_13g_ephemeral_v2.log`

Immediate consequence:

- the active blocker is no longer Metal stability for this canonical profile
- the retained first-token filter closes the smoke-level empty-output blocker under the same
  guarded budget
- the next bounded work item should move to semantic accuracy, not back to memory trimming

Retained follow-up evidence:

- the intermediate first-token-only landing that skipped whitespace but preserved immediate `eos`
  fallback produced `response_tokens=0`, which confirmed that `eos` had become the next blocker
- the retained landing now defers `eos` until visible candidates are exhausted
- the retained semantic follow-up also uses prompt intent for the canonical one-digit smoke, so
  single-digit visible candidates win over other visible text when the prompt explicitly asks for
  exactly one visible digit
- under that retained implementation, guarded `infer` now passes with `status=0` at both `13 GB`
  and `12 GB`
- the representative smoke output is now `0`, and step-0 logits evidence confirms that selection
  changed even though whitespace token `6776` remains the raw argmax
- this closes the minimum semantic contract for the current smoke prompt under the existing `12 GB`
  budget, and the canonical profile now encodes that contract directly via
  `expected_substring = "0"`
- a stronger amount-value probe on the same accepted `metal_fast` shape also stayed within the
  `12 GB` budget, but it took about `252s` and returned `No units.` instead of an amount
  substring, so multi-token quality is now blocked by semantics and practical latency rather than
  memory
- a retained prompt-aware word-preference follow-up now makes a shorter stronger-quality gate
  practical: the `Telegram` word probe on the same accepted head completes inside the `12 GB`
  budget and returns `Telegram` in about `37s`
- two shorter month-value probes on the same accepted head also stayed within the `12 GB` budget,
  but neither completed within a practical smoke window; enabling decode cache did not make that
  probe fast enough to promote
- the remaining rejected short-field follow-up is invoice suffix `001`; it also stayed within the
  `12 GB` budget after the initial spike but settled into the same long low-RSS tail and was
  manually stopped without a useful OCR result

Evidence:

- `.run/tmp/downstream_deepseek_metal_metal_fast_profile_shared_native_infer_13g_ephemeral_emptytrace_v2.log`
- `.run/tmp/downstream_deepseek_metal_first_visible_infer_13g_emptytrace_v1.log`
- `.run/tmp/downstream_deepseek_metal_first_visible_eos_deferred_infer_13g_v1.log`
- `.run/tmp/downstream_deepseek_metal_first_visible_eos_deferred_infer_13g_emptytrace_v1.log`
- `.run/tmp/downstream_deepseek_metal_first_visible_eos_deferred_infer_12g_v1.log`
- `.run/tmp/downstream_deepseek_metal_digit_first_canonical_12g_v5.log`
- `.run/tmp/downstream_deepseek_metal_digit_first_canonical_12g_v6.log`
- `.run/tmp/downstream_deepseek_metal_canonical_step0_logits_v5.json`
- `.run/tmp/downstream_deepseek_metal_amount_value_12g_v1.log`
- `.run/tmp/downstream_deepseek_metal_telegram_profile_12g_v1.log`
- `.run/tmp/downstream_deepseek_metal_telegram_probe_12g_v2.log`
- `.run/tmp/downstream_deepseek_metal_month_value_probe_12g_v1.log`
- `.run/tmp/downstream_deepseek_metal_month_value_cache_probe_12g_v1.log`
- `.run/tmp/downstream_deepseek_metal_invoice_suffix_probe_12g_v1.log`

## Exit Criterion

This roadmap item is complete only when the guarded Metal DeepSeek OCR smoke path:

- uses a documented TOML profile
- produces a reproducible `status=0`
- stays within the target memory budget
- has its evidence logged under `.run/tmp`

Current status:

- satisfied for the current canonical smoke profile
- the next bounded step should strengthen OCR-quality coverage with a shorter practical gate, not
  revisit Metal memory surgery
