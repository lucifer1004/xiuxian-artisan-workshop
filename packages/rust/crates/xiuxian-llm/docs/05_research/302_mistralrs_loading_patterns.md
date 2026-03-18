# Mistralrs Loading Patterns Relevant to DeepSeek OCR

:PROPERTIES:
:ID: llm-mistralrs-loading-patterns
:PARENT: [[../index.md]]
:STATUS: ACTIVE
:END:

## Working Question

If `DeepSeek OCR` still misses the guarded Metal budget by a narrow margin, should the next step be
another local tensor-lifetime tweak, a `candle-core` change, or a larger model-loading/runtime
pattern borrowed from `mistralrs-core`?

## Short Answer

`mistralrs-core` does not win primarily because of a single low-level Candle kernel change. The
largest reusable ideas are higher in the stack:

1. `mmap`-backed weight loading as the default path
2. load-time device placement through a device mapper
3. a Metal-oriented MoE fast backend that avoids per-expert loops and large scatter/combine staging

For `DeepSeek OCR`, item `1` already exists. The real missing pieces are `2` and especially `3`.

## Confirmed Findings

### 1. Mistralrs Defaults to `mmap` Weight Loading

`mistralrs-core` creates its `ShardedVarBuilder` through
`from_mmaped_safetensors(...)` and explicitly logs `Loading model using mmap strategy.` when the
fast path is active.

Primary source:

- `/Users/guangtao/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/mistralrs-core-0.7.0/src/utils/varbuilder_utils.rs`

Why it matters:

- this avoids eagerly materializing all tensors into an owned host-side map before model
  construction
- the builder can serve tensor loads directly from `MmapedSafetensors`

Why it is not the main missing optimization for `DeepSeek OCR`:

- vendored `deepseek-ocr` already uses `VarBuilder::from_mmaped_safetensors(...)`
- our own load traces already show `weights_mmap` as an explicit stage

Primary source:

- `/Users/guangtao/ghq/github.com/tao3k/omni-dev-fusion/third_party/deepseek-ocr/crates/infer-deepseek/src/model/mod.rs`

### 2. Mistralrs Places Tensors on Target Devices During Load

`mistralrs-core` threads a `DeviceMapper` through model loading. The mmap loader receives both:

- `layer_devices`
- `get_device_for_tensor`

This means placement decisions happen while tensors are being loaded, not only after a single
global device has already been chosen.

Primary sources:

- `/Users/guangtao/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/mistralrs-core-0.7.0/src/device_map.rs`
- `/Users/guangtao/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/mistralrs-core-0.7.0/src/pipeline/macros.rs`
- `/Users/guangtao/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/mistralrs-core-0.7.0/src/utils/varbuilder_utils.rs`

Why it matters:

- it reduces the need to load everything onto one device and then recast or move subsets later
- it creates a clear control point for host/device layer splits and topology-aware loading

Why it is only a secondary near-term target for `DeepSeek OCR`:

- our current bottleneck is a single-device Metal run, not a multi-device placement problem
- load-time device mapping may help with load spikes, but it does not directly address the current
  prefill-time MoE working-set wall

### 3. Mistralrs Has a Dedicated Metal MoE Fast Path

This is the biggest relevant finding.

`mistralrs-core` chooses `MoEExpertsBackend::Fast` for Metal. That path is explicitly described as
`gather-based implementation (good for Metal, ISQ)`.

Primary source:

- `/Users/guangtao/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/mistralrs-core-0.7.0/src/moe/experts.rs`

The `Fast` backend differs materially from the current vendored `deepseek-ocr` path:

- weights are loaded once into fused expert projections
- forward uses `gather_forward_autocast(...)`
- Metal avoids per-expert `Vec<Tensor>` accumulation
- Metal avoids `Tensor::cat -> scatter_set -> reshape -> weighted sum` over routed expert outputs

Instead, the fast path looks much closer to:

1. gather gate projection for routed experts
2. gather up projection
3. activation and pointwise combine
4. gather down projection
5. one weighted reduction across top-k experts

That is structurally different from the current `deepseek-ocr` `run_moe(...)` path, which still:

- reshapes all routed tokens
- sorts assignments
- loops expert-by-expert
- accumulates `outputs: Vec<Tensor>`
- builds `outs`
- builds `new_x`
- performs `scatter_set`
- performs a final combine

Primary source:

- `/Users/guangtao/ghq/github.com/tao3k/omni-dev-fusion/third_party/deepseek-ocr/crates/infer-deepseek/src/transformer/block.rs`

## Practical Reading for the Current DeepSeek OCR Work

The comparison narrows the next worthwhile optimization work:

- not `candle-core` first
- not another round of small prompt tweaks
- not another stack of boolean dtype toggles
- yes to a `DeepSeek OCR` MoE backend split that treats Metal as a first-class path

In other words:

- `mmap` is already present, so duplicating that work will not move the main wall
- `device_map` is worth studying for load placement, but it is not the clearest answer to the
  current prefill MoE overage
- the strongest reusable idea is `MoEExpertsBackend::Fast`

## Immediate Design Implication

If we continue borrowing from `mistralrs-core`, the next architectural target should be:

1. introduce a dedicated Metal-oriented MoE backend inside vendored `deepseek-ocr`
2. pre-pack or fuse expert projections at load time
3. replace the current loop + `Vec<Tensor>` + scatter/combine path for Metal prefill

That is the first direction that plausibly explains a large step change, rather than another
single-digit-MB trim.

## Current Landing State

The first useful landing is now in place inside vendored `deepseek-ocr`:

- `MoeWeights` is no longer implicitly tied to a single execution layout
- the code now has an explicit MoE backend seam
- the default backend remains `slow`
- a `metal_fast` backend skeleton exists, but it currently falls back to the slow helper
- the `metal_fast` branch now also carries a first packed-expert carrier for eager float experts

Why this matters:

- future Metal-fast experiments can be isolated behind one backend branch
- the existing slow path remains the default reference path
- guarded regressions can be attributed to the new backend branch instead of to unrelated edits

What has **not** happened yet:

- no gather-style fast MoE forward has landed
- no pre-packed fused expert weights have landed
- no default behavior change has landed

The new packed carrier is deliberately narrow:

- it is only prepared for eager float experts
- deferred or non-float experts still force fallback behavior
- the `metal_fast` forward branch now uses the packed routed-expert MLP path when the carrier is
  available

That makes the next step precise: wire the first gather-style forward or fused-expert read path to
the new carrier instead of inventing another backend seam.

## First Branch Reality Check

The first real `metal_fast` probe is now informative:

- the stable lazy-expert guarded profile does **not** exercise the packed branch, because
  `lazy_moe_experts=true` prevents pack creation
- a manual eager-expert guarded probe with `XIUXIAN_VISION_MOE_BACKEND=metal_fast` and
  `XIUXIAN_VISION_LAZY_MOE_EXPERTS=0` does exercise the new branch
- that eager probe still exceeds the `12 GB` guard, landing around `12.13 GB` in roughly `6.4s`

Interpretation:

- the packed routed-expert carrier is a useful structural landing
- but simply replacing per-expert `DenseMlpWeights` reads with packed gate/up/down tensors is not
  yet enough
- the next meaningful step still has to remove more of the slow-path shape, especially the
  routed-expert loop and the downstream scatter/combine working set

Follow-up result:

- the `metal_fast` branch now also has a token-major packed routed-expert path that removes the
  explicit `Vec<Tensor>` accumulation and `scatter_set` combine shape
- the first guarded eager-expert probe with that token-major path still misses the budget and
  actually regresses slightly, landing around `12.23 GB`

This tightens the diagnosis again:

- `Vec<Tensor>` and `scatter_set` were not the only meaningful wall
- the remaining cost is likely earlier in the branch:
  eager packed-weight materialization and/or the per-slot packed gather matmul shape itself

Second follow-up result:

- the `metal_fast` branch now also has a grouped-by-expert packed routed-expert path
- instead of selecting packed expert weights once per token/slot pair, it batches token positions
  by expert and executes each packed expert once per grouped slice
- the guarded eager-expert probe for that grouped path still misses the `12 GB` budget, but it
  improves materially to roughly `12.08 GB` in about `24.7s`
- evidence is recorded in
  `.run/tmp/downstream_deepseek_metal_metal_fast_grouped_probe_fresh.log`

This narrows the diagnosis again:

- the slot-wise packed gather/matmul shape was part of the problem after all
- grouping token positions by expert reduces the peak enough to be measurable
- but the remaining overage is still real, so the next wall is not solved by routing-shape changes
  alone
- the likely remaining costs are eager packed-weight retention and the rest of the routed/shared
  expert working set around the grouped path

Rejected follow-up:

- removing `contiguous()` from packed expert-weight selection looked attractive as a way to avoid
  an extra materialization step
- the guarded eager-expert probe with that change regressed instead, reaching roughly `12.20 GB`
  in about `13.5s`
- evidence is recorded in
  `.run/tmp/downstream_deepseek_metal_metal_fast_grouped_viewprobe.log`

This is a useful negative result:

- the current packed path appears to rely on explicit materialization for a better execution shape
- keeping the packed-weight view non-contiguous does not buy headroom here
- the change was reverted, so the grouped-by-expert path remains the current `metal_fast` head

Rejected follow-up:

- deferring expert-pack construction from load time to runtime also looked attractive, because it
  could have reduced eager packed-weight retention during the guarded Metal probe
- the experiment disabled load-time prepack and rebuilt the same eager guarded probe with runtime
  packing instead
- that branch regressed, landing around `12.10 GB` in roughly `6.7s`
- evidence is recorded in
  `.run/tmp/downstream_deepseek_metal_metal_fast_runtimepack_probe.log`

This removes another plausible branch from the search space:

- load-time prepack is currently the better head for `metal_fast`
- moving pack construction into the forward path does not recover the missing headroom
- the grouped-by-expert, load-time-prepacked branch remains the active reference point

Confirmed follow-up:

- `metal_fast` was then tightened so it no longer builds slow-path-only routing staging when the
  fast branch is selected
- this removes eager construction of slow-only tensors such as `sorted_tokens` and `idxs` from
  the fast branch
- the same eager guarded probe improved again, now landing around `12.03 GB`
- evidence is recorded in
  `.run/tmp/downstream_deepseek_metal_metal_fast_minimal_staging_probe.log`

This is a more meaningful result than the rejected pack-timing branches:

- the remaining overage is now only a few tens of megabytes
- the fast branch does improve when it stops paying for slow-path routing artifacts
- the next wall is likely the residual fast-path working set itself:
  slot-level combine and/or shared-expert execution

Rejected follow-up:

- replacing the per-slot `slot_out -> weighted -> combined.add(...)` flow with a direct
  `scatter_add_set` accumulation path looked attractive because it should have removed two
  full-matrix temporaries from the fast branch
- the guarded eager probe regressed instead, landing around `12.37 GB` in about `8.1s`
- evidence is recorded in
  `.run/tmp/downstream_deepseek_metal_metal_fast_scatter_add_probe.log`

This removes yet another tempting rewrite:

- the current Metal backend does not benefit from this `scatter_add_set` combine shape
- the `12.03 GB` minimal-staging branch remains the active head
- the next diagnosis should shift away from slot-level accumulation rewrites and toward the
  remaining expert-side working set, especially the shared-expert path

Confirmed follow-up:

- the guarded Metal probe was rerun with explicit TOML-backed `metal_fast` eager profiles so the
  backend selector and eager-MoE policy are visible in the runner log instead of being implicit
  shell state
- the explicit eager baseline reached roughly `14.29 GB`, recorded in
  `.run/tmp/downstream_deepseek_metal_metal_fast_profile_baseline.log`
- a paired diagnostic run with `skip_shared_experts = true` reached roughly `13.43 GB`, recorded
  in `.run/tmp/downstream_deepseek_metal_metal_fast_profile_skip_shared.log`

This does not make `skip_shared_experts` a valid product path, but it does provide a usable
directional result:

- `shared_experts` account for roughly `0.86 GB` of the current eager `metal_fast` overshoot
- the next optimization should target shared-expert execution shape or dtype before reopening
  routed-expert combine rewrites
- to preserve behavior while testing that hypothesis, the codebase now carries a separate
  `XIUXIAN_VISION_SHARED_EXPERT_F32_COMPUTE` toggle so shared-expert compute can be lowered
  independently from routed experts

Confirmed follow-up:

- the first behavior-preserving shared-expert experiment was then run with
  `XIUXIAN_VISION_SHARED_EXPERT_F32_COMPUTE=0`
- after rebuilding an isolated real-Metal test binary and rerunning the guarded probe, the best
  result improved again to roughly `12.12 GB`
- a second repeat still landed above budget at roughly `12.33 GB`, so this branch is close but
  not yet stable within the `12 GB` gate
- evidence is recorded in
  `.run/tmp/downstream_deepseek_metal_metal_fast_profile_shared_native_post_revert.log`
  and `.run/tmp/downstream_deepseek_metal_metal_fast_profile_shared_native_repeat.log`

This is the strongest bounded improvement on the shared side so far:

- shared-expert compute dtype, not just shared-expert presence, is a meaningful part of the
  overshoot
- the remaining gap is now on the order of `0.12-0.33 GB`, depending on run-to-run variance
- the next shared-side experiment should continue from this branch, not from `skip_shared`

Rejected follow-up:

- forcing the final shared residual add to stay native instead of using the existing stable
  F32 add was tested as a bounded follow-up
- that branch regressed immediately, reaching roughly `12.83 GB`
- evidence is recorded in
  `.run/tmp/downstream_deepseek_metal_metal_fast_profile_shared_native_residual_native.log`

This removes another tempting shortcut:

- the stable residual add should stay in place for now
- the accepted head is `shared_expert_f32_compute = false`, not a native residual add rewrite

Rejected follow-up:

- reusing the token-major routed input (`work.tokens`) for shared-expert MLP execution looked like
  a plausible way to avoid one more shared-side reshape/contiguous path
- after rebuilding an isolated real-Metal test binary and rerunning the same guarded profile, the
  result still landed around `12.15 GB`
- evidence is recorded in
  `.run/tmp/downstream_deepseek_metal_metal_fast_profile_shared_native_token_reuse.log`

This is not a convincing win:

- it does not beat the current accepted head (`~12.12 GB` best observed, `~12.33 GB` repeat)
- it adds code motion without a stable memory gain
- the change was reverted, so the current head remains the simpler shared-input path with
  `shared_expert_f32_compute = false`

Rejected follow-up:

- deferring only `shared_experts` at load time was tested as a bounded attempt to reduce the
  eager `metal_fast` load spike without changing routed-expert execution
- the load-only guard did improve materially, dropping from roughly `12.72 GB` to roughly
  `12.30 GB`, recorded in
  `.run/tmp/downstream_deepseek_metal_metal_fast_profile_shared_lazy_load_only.log`
- the real infer probe regressed to roughly `12.58 GB`, recorded in
  `.run/tmp/downstream_deepseek_metal_metal_fast_profile_shared_lazy_infer.log`
- because the full infer path regressed, `lazy_shared_experts` was removed from the tree after
  verification

This narrows the diagnosis again:

- shared-expert load residency is part of the load spike, but it is not a free win
- first-use shared materialization shifts cost into the infer path and loses against the current
  accepted head
- the retained head stays `shared_expert_f32_compute = false` with eager shared weights

Rejected follow-up:

- keeping packed routed-expert weights as non-contiguous views inside the `metal_fast` packed
  helpers was tested as another bounded attempt to shave one more eager materialization from the
  accepted shared-native head
- after rebuilding an isolated real-Metal binary and rerunning the same guarded profile, the
  probe regressed to roughly `12.35 GB`
- evidence is recorded in
  `.run/tmp/downstream_deepseek_metal_metal_fast_profile_shared_native_weight_view.log`

This closes that micro-optimization:

- the accepted head is still `shared_expert_f32_compute = false`
- packed-weight view shape is not the remaining win
- the next bounded work should stay on fast-path working-set variance instead of revisiting
  packed-weight view materialization

Rejected follow-up:

- replacing the packed `metal_fast` carrier with load-time contiguous per-expert 2D weights was
  tested as a more structural attempt to remove runtime expert-weight materialization entirely
- after rebuilding the isolated real-Metal binary and rerunning the accepted shared-native
  guarded profile, the first probe landed around `12.20 GB`
- a repeat spiked much higher to roughly `13.14 GB`
- evidence is recorded in
  `.run/tmp/downstream_deepseek_metal_metal_fast_profile_shared_native_expert2d.log`
  and `.run/tmp/downstream_deepseek_metal_metal_fast_profile_shared_native_expert2d_repeat.log`

This keeps the accepted head unchanged:

- the grouped routed-expert schedule is still useful, but this carrier rewrite does not produce a
  better guarded result than the current stacked-pack head
- the change was reverted after verification
- the remaining gap is still better treated as working-set variance inside the existing carrier,
  not a carrier replacement win

Rejected follow-up:

- stacking one more boolean on top of the accepted head was re-tested after `metal_fast` and
  `shared_expert_f32_compute = false` were both already in place
- specifically, the same guarded profile was rerun with `moe_expert_f32_compute = false` added on
  top of the accepted shared-native head
- the guarded result still landed around `12.22 GB`, which is worse than the current best
  `~12.12 GB`
- evidence is recorded in
  `.run/tmp/downstream_deepseek_metal_metal_fast_profile_shared_native_native_moe.log`

This closes that stack-up question for the current architecture:

- `metal_fast + shared_native + native_moe` is not a better head
- the transient test profile was removed after verification
- the next bounded work should stay on working-set variance inside the accepted head instead of
  reopening the routed-expert native-compute toggle

## Accepted-Head Diagnostics Now Resolve the Last Visible Load Stage

The latest useful landing was in observability rather than in memory reduction:

- `DeepseekOcrModel::load(...)` now emits stage lines at load entry, after config, and just before
  `weights_mmap`
- `scripts/run_real_metal_test.py` can now carry those lines reliably under the guarded harness
  when `XIUXIAN_VISION_STAGE_TRACE_STDERR=1`

For the current accepted head:

- `metal_fast`
- eager routed experts
- `shared_expert_f32_compute = false`

the new guarded evidence changes the diagnosis:

- in a guarded `load` repro, the trace now crosses `weights_mmap.completed`,
  `deferred_moe_source.completed`, and `language.started`
- in a guarded `infer` repro, the trace also crosses those same post-mmap boundaries
- the latest widened `load` traces now go past the old layer-start and gate boundaries
- with finer MoE and linear-loader tracing enabled, the remaining `load` wall is attributable
  inside MoE expert linear materialization
- the latest representative `load` probe dies while materializing
  `model.layers.5.mlp.experts.59.up_proj.weight`
- the guarded `infer` trace reaches `deepseek.language.transformer_layer.start` for `layer_idx=5`
  after completing layer `4`

Evidence:

- `.run/tmp/downstream_deepseek_metal_metal_fast_profile_shared_native_trace_pty_load_15g_v6.log`
- `.run/tmp/downstream_deepseek_metal_metal_fast_profile_shared_native_trace_pty_13g_v3.log`

Confirmed follow-up:

- eager routed-expert `LinearWeights::load` now keeps view-backed weights when the accepted
  `metal_fast` branch is building a packable eager expert layout
- contiguous materialization for those routed experts was moved to the explicit `metal_fast`
  pack path instead of happening once in `LinearWeights::load` and again in the pack builder
- the code now emits explicit pack-boundary traces:
  - `deepseek.language.transformer_layer.mlp.moe.pack.start`
  - `...pack.gate.completed`
  - `...pack.up.completed`
  - `...pack.down.completed`
  - `...pack.completed`

This changes the diagnosis materially:

- the widened guarded `load` run no longer dies during eager routed-expert linear materialization
- the accepted-head widened `load` run now reaches `deepseek.language.weights_ready` and exits
  successfully under the `15 GB` guard
- the guarded `infer` run still misses the `13 GB` budget, but it now lives much longer and dies
  materially later than the earlier accepted-head baseline
- the old accepted-head `infer` trace died in about `1.2s`, immediately after layer `4`, at about
  `13.29 GB`
- the new accepted-head `infer` trace dies in about `18.9s`, at about `13.04 GB`, while loading
  layer `5` routed experts

Evidence:

- `.run/tmp/downstream_deepseek_metal_metal_fast_profile_shared_native_trace_pty_load_15g_v7.log`
- `.run/tmp/downstream_deepseek_metal_metal_fast_profile_shared_native_trace_pty_13g_v4.log`
- `.run/tmp/downstream_deepseek_metal_metal_fast_profile_shared_native_trace_pty_13g_v3.log`

Rejected follow-up:

- a direct routed-expert projection-pack rewrite was tested next
- instead of loading eager routed experts into `DenseMlpWeights` and letting `from_slow(...)`
  stack them, the branch loaded gate/up/down one projection at a time and wrote them directly into
  a preallocated packed carrier
- the intent was to remove the transient "whole expert struct plus packed copy" residency during
  widened guarded `load`
- the guarded widened `load` result regressed: the new probe died at about `15.05 GB` during
  layer `6` routed-expert `up_proj/down_proj` fetch, instead of matching the retained `v7` run
  that reached `deepseek.language.weights_ready`
- the branch was reverted, and the accepted head remains the earlier retained materialization trim

Evidence:

- `.run/tmp/downstream_deepseek_metal_metal_fast_profile_shared_native_trace_pty_load_15g_v8.log`

This narrows the next bounded work again:

- mmap is still resolved
- contiguous expert-weight materialization is no longer the active widened-load wall
- pack construction itself is no longer the active widened-load wall
- the remaining accepted-head overshoot is now better described as eager routed-expert
  `weight.get(...)` residency during the infer-side load path, especially once layer `5` begins
- the next investigation should stay inside eager expert fetch/materialization order for the
  existing `metal_fast + shared_expert_f32_compute = false` head, not reopen earlier toggle
  branches and not pivot to `candle-core`

13 GB parity follow-up:

- a like-for-like accepted-head `phase=load --max-rss=13` trace was captured next to compare
  pure load against the existing accepted-head `phase=infer --max-rss=13` evidence
- pure `load` does not pass the `13 GB` guard either
- the new pure-load wall appears earlier than the current infer wall:
  - the `13 GB` pure-load run dies in layer `4` while loading `shared_experts`
  - the older accepted-head `13 GB` infer run dies later, in layer `5` routed-expert fetch
- this means the remaining `13 GB` problem is not infer-only overhead
- it also means shared-expert load residency is still an active contributor under the target guard,
  even though the widened `15 GB` load path reaches `weights_ready`

Evidence:

- `.run/tmp/downstream_deepseek_metal_metal_fast_profile_shared_native_trace_pty_load_13g_v1.log`
- `.run/tmp/downstream_deepseek_metal_metal_fast_profile_shared_native_trace_pty_13g_v4.log`

Shared-expert attribution follow-up:

- `skip_shared_experts` originally only affected forward execution; the load path still materialized
  `shared_experts`, which made the earlier `13 GB` comparison ambiguous
- the load path now honors the same existing diagnostic flag, without changing the default head or
  introducing a new toggle
- with `skip_shared_experts = true`, the widened `phase=load --max-rss=13` repro no longer dies in
  `shared_experts`; it now dies earlier in the same layer `4`, during routed-expert eager loads
  around expert `45`
- the matching `phase=infer --max-rss=13` repro also skips `shared_experts` at load time and still
  dies in layer `4`, during routed-expert eager loads around expert `38`
- this means `shared_experts` are a real contributor to the remaining `13 GB` miss, but they are
  not the full wall; once removed from the load path, the next active boundary is still eager
  routed-expert `weight.get(...)` residency in layer `4`

Evidence:

- `.run/tmp/downstream_deepseek_metal_metal_fast_profile_skip_shared_trace_pty_load_13g_v2.log`
- `.run/tmp/downstream_deepseek_metal_metal_fast_profile_skip_shared_trace_pty_13g_v1.log`

Lazy routed-expert best-case follow-up:

- a manual repo-local diagnostic was run with the same accepted-head shape as the current
  `skip_shared` probe, but with only one extra difference: `XIUXIAN_VISION_LAZY_MOE_EXPERTS=1`
- the `phase=load --max-rss=13` run then passed cleanly and reached `deepseek.language.weights_ready`
- that widened-best-case load repro finished in about `13.6s`, with observed RSS around `6.62 GB`
- a matching `phase=infer --max-rss=13` no-trace repro was then allowed to run for `86s`; it never
  approached the budget wall, with observed RSS staying at or below about `7.57 GB`
- this is not yet an accepted product head, but it is now a strong attribution result:
  the remaining `13 GB` wall is dominated by eager routed-expert residency, not by shared experts,
  not by mmap, and not by generic decode overhead

Evidence:

- `.run/tmp/downstream_deepseek_metal_manual_lazy_skip_shared_trace_pty_load_13g_v2.log`
- `.run/tmp/downstream_deepseek_metal_manual_lazy_skip_shared_infer_13g_notrace_v1.log`

Retained `metal_fast` routed-expert deferral landing:

- the retained canonical profile is still
  `deepseek_metal_smoke_12g_safe384_digit1_native_inputs_native_attn_native_gate_inputs_metal_fast_eager_shared_native`
- no new TOML profile or env toggle was added
- instead, `transformer/weights.rs` now chooses an internal routed-expert load strategy; under
  `MoeExecutionBackend::MetalFast`, with a deferred source and no snapshot, routed experts now
  stay deferred even when the external `lazy_moe_experts` flag is still off
- shared experts remain eager, so the retained profile shape is unchanged at the surface
- the widened `phase=load --max-rss=15` repro now passes cleanly and reaches
  `deepseek.language.weights_ready` in about `14.1s`
- the guarded `phase=infer --max-rss=13` repro still misses the target budget, but it no longer
  dies in model-load residency; it reaches `deepseek.language.weights_ready`, enters
  `block.forward.moe`, and only then dies at roughly `13.02 GB` after about `40.4s`
- the new last visible stage is inside deferred routed-expert forward execution, around
  `block.forward.moe.expert.started layer_idx=3 expert_idx=27`, not inside eager routed-expert
  load-time `weight.get(...)`

Evidence:

- `.run/tmp/downstream_deepseek_metal_metal_fast_profile_shared_native_trace_pty_load_15g_v9.log`
- `.run/tmp/downstream_deepseek_metal_metal_fast_profile_shared_native_trace_pty_13g_v5.log`

That means the codebase is now structurally ready for the real MoE fast-path work, but the active
budget wall has moved out of load-time routed-expert residency and into deferred routed-expert
forward materialization.

Follow-up landing on that new wall:

- deferred routed experts now keep their `gate/up/down` projection tensors view-backed during
  `DeferredDenseMlpWeights::materialize(...)` instead of eagerly forcing all three projections
  contiguous up front
- this does not add any new profile or external toggle; it only changes the internal
  `metal_fast + deferred routed expert` materialization path
- the widened `phase=load --max-rss=15` run still passes comfortably, now finishing in about
  `5.0s` with observed RSS topping out around `10.36 GB`
- more importantly, the guarded no-trace `phase=infer --max-rss=13` run no longer fails on memory
  at all; it completes inference in about `32.7s`, stays under budget with observed RSS peaking
  around `9.19 GB`, and fails only because OCR output is empty

Retained follow-up:

- the canonical Metal profile first moved from OOM to a single-token empty-output failure
- the empty-output trace pinned that failure to whitespace token `6776`
- a narrow retained fix now changes only first-token selection: whitespace-only decoded candidates
  are skipped, and `eos` is deferred until no visible candidate remains

New evidence:

- `.run/tmp/downstream_deepseek_metal_metal_fast_profile_shared_native_load_15g_ephemeral_v2.log`
- `.run/tmp/downstream_deepseek_metal_metal_fast_profile_shared_native_infer_13g_ephemeral_v2.log`
- `.run/tmp/downstream_deepseek_metal_first_visible_infer_13g_emptytrace_v1.log`
- `.run/tmp/downstream_deepseek_metal_first_visible_eos_deferred_infer_13g_v1.log`
- `.run/tmp/downstream_deepseek_metal_first_visible_eos_deferred_infer_12g_v1.log`

This changes the active diagnosis again:

- the retained canonical Metal profile is no longer blocked on memory
- the retained first-token filter removes the whitespace-token blocker from the canonical path
- the guarded `infer` run now passes with `status=0` at both `13 GB` and `12 GB`
- the retained digit-first follow-up now chooses token `18` (`0`) at step `0` even though the raw
  logits still rank whitespace token `6776` first and `eos` second
- the representative smoke output is now `0`, so the canonical one-digit smoke is no longer
  blocked on minimal prompt-level semantics either
- a stronger multi-token amount-value probe also stays inside the `12 GB` guard, but it took about
  `252s` and returned `No units.` instead of an amount substring; the next quality wall is now
  semantic correctness and practical latency, not memory
- two shorter month-value probes also stayed inside the `12 GB` guard, but neither completed in a
  practical smoke window, regardless of whether decode cache was disabled or enabled

## Recommendation

The next bounded investigation should move from Metal stability and first-token semantics to richer
OCR quality coverage:

`Which stronger OCR-quality probe stays small enough to act as a real smoke gate, now that amount-value probes are semantically wrong and both cache and no-cache month-value probes are still too slow under the same 12 GB budget?`

`candle-core` should remain out of scope.
