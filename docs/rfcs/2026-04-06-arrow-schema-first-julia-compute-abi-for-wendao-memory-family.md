---
type: knowledge
title: "RFC: Arrow Schema-First Julia Compute ABI for the Wendao Memory Family"
category: "rfc"
status: "draft"
authors:
  - codex
created: 2026-04-06
tags:
  - rfc
  - wendao
  - memory
  - julia
  - arrow
  - abi
  - flight
metadata:
  title: "RFC: Arrow Schema-First Julia Compute ABI for the Wendao Memory Family"
---

# RFC: Arrow Schema-First Julia Compute ABI for the Wendao Memory Family

## 1. Summary

This RFC defines the first family-level Julia compute ABI for the Wendao memory
stack.

The governing principle is:

**Julia owns high-performance scientific and mathematical compute only. Rust
remains the authoritative owner of host state, lifecycle, registry, fallback,
audit, and final mutation decisions.**

This RFC extends the existing scorer-style Arrow contract precedent into one
`memory` family with four profiles:

1. `episodic_recall`
2. `memory_gate_score`
3. `memory_plan_tuning`
4. `memory_calibration`

The primary decisions are:

1. the lane is a `memory-family Julia compute ABI`, not a `memory-first ABI`
2. `WendaoArrow.jl` becomes the Julia-side transport, contract, and authoring
   substrate for compute plugins only
3. Rust remains the authoritative owner of memory state, lifecycle, fallback,
   audit, and final mutations
4. memory-family Julia services consume read-only host projections rather than
   internal Rust memory structs
5. recommendation-producing profiles remain recommendation-only until Rust
   explicitly commits a change

## 2. Alignment

This RFC aligns with the following stable references:

1. [architecture.md](../01_core/memory/architecture.md)
2. [2026-04-05-wendao-memory-layer-boundaries-rfc.md](./2026-04-05-wendao-memory-layer-boundaries-rfc.md)
3. [SPEC.md](../01_core/wendao/SPEC.md)

This RFC also builds on the existing scorer-style Arrow schema contract
documented by the WendaoArrow package, but canonical RFCs in this repository do
not link directly to hidden workspace paths.

The paired execution tracking for this RFC follows an active ABI blueprint and
ExecPlan, but canonical docs do not link hidden tracking paths directly.

## 3. Problem Statement

Wendao now has a clear memory ownership boundary, but it still lacks a formal
ABI for Julia-owned compute over the memory family.

Without that ABI, architecture drift is likely:

1. Julia packages may gradually absorb host authority instead of remaining
   compute-only
2. Rust host code may grow one bespoke adapter per Julia package
3. physical Arrow schemas and semantic profile meaning may drift together
   without explicit version boundaries
4. shadow-mode rollout may exist as a switch but not as an auditable
   observability surface

The system already contains the correct ownership foundation:

1. `xiuxian-memory-engine` is an episodic operational memory core
2. `WendaoArrow.jl` is a Julia transport and contract layer, not a host-policy
   layer
3. `xiuxian-wendao-runtime` owns generic Flight negotiation
4. `xiuxian-wendao-julia` owns Julia-specific manifest discovery and thin typed
   decoding

This RFC formalizes how those pieces compose into a stable memory-family Julia
compute ABI.

## 4. Allowed Julia Compute Scope

Julia compute services in the memory family MAY own:

1. scoring
2. reranking
3. uncertainty estimation
4. calibration
5. optimization
6. threshold fitting
7. solver and structural math
8. scenario-specific scientific models

Julia compute services in the memory family MUST NOT own:

1. host state mutation
2. lifecycle transitions
3. registry authority
4. durable knowledge promotion
5. fallback policy ownership
6. final audit or event ownership

This boundary is normative for `WendaoArrow.jl`, `WendaoMemory.jl`, and any
future Julia memory-family package.

## 5. Target Architecture

### 5.1 Ownership Model

The target ownership model is:

1. `xiuxian-memory-engine` owns authoritative memory state, lifecycle
   transitions, state backend, event emission, and fallback/local
   implementation
2. `xiuxian-wendao-runtime` owns generic Flight client, route normalization,
   timeout policy, negotiation, and batch transport
3. `xiuxian-wendao-julia` owns only Julia-specific manifest discovery, route
   defaults, typed validation, typed decoding, and minimal binding glue
4. `WendaoArrow.jl` owns the reusable Julia-side transport, contract, manifest,
   service declaration, and validation substrate for compute plugins only
5. Julia memory-family packages own memory-family compute only

### 5.2 `WendaoArrow.jl` Role

`WendaoArrow.jl` becomes the Julia-side transport, contract, and authoring
substrate for compute plugins.

It MAY own:

1. schema builders
2. manifest builders
3. service declaration helpers
4. request and response validation helpers

It MUST NOT own:

1. lifecycle policy
2. state mutation
3. registry authority
4. fallback policy
5. host governance

This RFC explicitly rejects a standalone `PluginKit.jl`.

### 5.3 Runtime-Level Integration

The integration surface is runtime-level rather than repo-plugin-level.

The canonical host surface is:

```toml
[memory.julia_compute]
enabled = true
base_url = "grpc://127.0.0.1:18825"
schema_version = "v1"
timeout_secs = 3
fallback_mode = "rust"
shadow_compare = true

[memory.julia_compute.routes]
episodic_recall = "/memory/episodic_recall"
memory_gate_score = "/memory/gate_score"
memory_plan_tuning = "/memory/plan_tuning"
memory_calibration = "/memory/calibrate"
```

This config is intentionally host-owned. It does not move authority into Julia.

## 6. ABI Layers

The memory-family Julia compute ABI is defined by three layers.

### 6.1 Physical Arrow Schema

The physical layer defines:

1. column names
2. Arrow data types
3. nullability
4. additive-column rules
5. batch semantics

### 6.2 Semantic Contract

The semantic layer defines:

1. join keys
2. row-order guarantees or lack thereof
3. duplicate handling
4. score, verdict, or confidence meaning
5. fallback triggers

### 6.3 Capability Contract

The capability layer defines:

1. `family`
2. `capability_id`
3. `profile_id`
4. `request_schema_id`
5. `response_schema_id`
6. `schema_version`
7. `route`
8. `health_route`
9. `timeout_secs`
10. `scenario_pack`

## 7. Versioning Semantics

The ABI MUST distinguish physical transport versioning from profile semantics.

Rules:

1. `schema_version` is the physical or transport contract version
2. `profile_id + request_schema_id + response_schema_id` define the semantic
   profile contract
3. a semantic meaning change is breaking even when the physical columns remain
   identical
4. additive physical columns remain allowed only when the receiving host path
   does not require them

This rule exists so a stable Arrow column shape does not conceal a semantic
profile break.

## 8. Capability Manifest Shape

The first standardized family is one umbrella `memory` family.

Each manifest entry MUST carry:

1. `family = "memory"`
2. `capability_id`
3. `profile_id`
4. `request_schema_id`
5. `response_schema_id`
6. `route`
7. `schema_version`
8. `enabled`

It MAY also carry:

1. `health_route`
2. `timeout_secs`
3. `scenario_pack`

Rust host compatibility should scale by family/profile. It must not require a
new host business adapter for every Julia package that registers a memory
profile.

## 9. Host Read Model

Julia memory-family services never consume `EpisodeStore` internals directly.

The canonical data path is:

1. Rust host reads authoritative memory state
2. Rust host materializes a canonical read-only projection batch or snapshot
3. Julia compute runs only on that projection
4. Rust host interprets results and decides whether to mutate state

This read model prevents Rust internal storage details from becoming part of
the Julia ABI.

## 10. Invariants

The following invariants are normative.

### 10.1 Compute-Only Ownership

Julia compute services MUST NOT mutate authoritative host memory state.

### 10.2 Lifecycle Authority Remains in Rust

Julia compute services MUST NOT own lifecycle transitions.

### 10.3 No Durable Promotion Authority

Julia compute services MUST NOT directly govern durable knowledge promotion.

### 10.4 Read-Only Projection First

Rust host MUST materialize read-only projection batches before Julia compute is
invoked.

### 10.5 Recommendation Before Mutation

Any Julia-side result that implies a state change MUST be treated as
recommendation-only until Rust host commits it.

### 10.6 No Hidden Writeback Channel

Julia compute results MUST NOT trigger host mutation through hidden metadata,
side-channel flags, or implicit response conventions.

## 11. Minimal Compat Principle

Rust host compatibility must remain family-level rather than package-level.

Therefore:

1. Rust host MUST NOT add one bespoke adapter per Julia package
2. Rust host SHOULD implement family-level adapters only
3. new Julia packages inside an existing family/profile SHOULD require
   manifest/schema additions only, not new host business adapters
4. `xiuxian-wendao-julia` MUST remain ecosystem-thin and MUST NOT grow a
   second host-local business adapter layer

## 12. Memory Family Profiles

### 12.1 `episodic_recall`

Definition:
retrieval profile over a Rust-owned read-only memory projection.

Allowed Julia responsibilities:

1. retrieval compute
2. reranking
3. uncertainty estimation
4. scenario-aware ranking

Forbidden Julia responsibilities:

1. persistence
2. lifecycle mutation
3. state writes

### 12.2 `memory_gate_score`

Definition:
recommendation-only gate scoring profile.

It returns:

1. score
2. confidence
3. suggested verdict
4. reason

It MUST NOT cause a lifecycle transition on its own.

### 12.3 `memory_plan_tuning`

Definition:
advice-only tuning profile.

It returns:

1. parameter recommendations
2. budget recommendations
3. confidence and rationale

It MUST NOT mutate active host config directly.

### 12.4 `memory_calibration`

Definition:
artifact/recommendation-only calibration profile.

It returns:

1. thresholds
2. weights
3. calibration artifacts
4. summary metrics

It MUST NOT auto-activate calibrated outputs.

## 13. Canonical Schema Fragments

Profile schemas SHOULD be composed from canonical fragments plus additive
profile-specific columns.

### 13.1 `identity_fragment`

Canonical identity fields such as:

1. `row_id`
2. `candidate_id`
3. `scope`

### 13.2 `score_fragment`

Canonical score fields such as:

1. `semantic_score`
2. `utility_score`
3. `final_score`
4. `confidence`

### 13.3 `verdict_fragment`

Canonical decision fields such as:

1. `verdict`
2. `reason`
3. `next_action`

### 13.4 `tuning_fragment`

Canonical tuning fields such as:

1. `k1`
2. `k2`
3. `lambda`
4. `min_score`
5. `max_context_chars`

### 13.5 `memory_projection_fragment`

Canonical read-only memory input fields such as:

1. `intent_embedding`
2. `q_value`
3. `success_count`
4. `failure_count`
5. `retrieval_count`
6. `created_at_ms`
7. `updated_at_ms`
8. `scope`

This fragment exists so memory-family profiles do not reinvent their core
read-only episode features route by route.

## 14. Shadow Compare and Observability

When `shadow_compare = true`, the host path SHOULD record at minimum:

1. recall rank drift
2. gate verdict drift
3. confidence drift
4. timeout rate
5. fallback rate
6. schema validation failure rate

The purpose of shadow compare is operational evidence, not silent replacement
of the Rust path.

## 15. Non-Goals

This RFC does not:

1. scaffold `.data/WendaoMemory.jl`
2. implement runtime execution seams
3. move authoritative memory state into Julia
4. standardize all possible capability families in one pass
5. create a standalone `PluginKit.jl`

## 16. Final Decisions

The final decisions are:

1. the lane is a `memory-family Julia compute ABI`
2. Julia owns compute only
3. Rust owns authority
4. `WendaoArrow.jl` absorbs the authoring substrate for compute plugins only
5. memory-family services consume read-only Rust projections
6. recommendation-producing profiles remain recommendation-only until Rust
   commits a change
7. compatibility grows by family/profile rather than by Julia package count
