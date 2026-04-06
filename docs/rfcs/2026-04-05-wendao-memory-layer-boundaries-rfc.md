---
type: knowledge
title: "RFC: Wendao Memory Layer Boundaries"
category: "rfc"
status: "draft"
authors:
  - codex
created: 2026-04-05
tags:
  - rfc
  - wendao
  - memory
  - knowledge
  - cache
  - strategy
  - layering
metadata:
  title: "RFC: Wendao Memory Layer Boundaries"
---

# RFC: Wendao Memory Layer Boundaries

## 1. Summary

This RFC defines the formal boundary between four asset layers in the Wendao
stack:

1. `CacheArtifact`
2. `EpisodicMemory`
3. `WorkingKnowledge`
4. `DurableKnowledge`

The primary decision is:

1. `xiuxian-memory-engine` is an episodic operational memory core only
2. `EpisodeStore` must not become a generic cache, strategy, or knowledge
   registry
3. validated `SearchStrategyFlow` belongs to `WorkingKnowledge` by default
4. durable docs, projected pages, and curated long-lived notes belong to
   `DurableKnowledge`
5. `MemoryGate` must not directly govern durable publication or durable purge

## 2. Alignment

This RFC aligns with the following stable references:

1. [architecture.md](../01_core/memory/architecture.md)
2. [SPEC.md](../01_core/wendao/SPEC.md)
3. [trinity-control.md](../01_core/omega/trinity-control.md)
4. [research-memrl-vs-omni-memory.md](../02_dev/workflows/research-memrl-vs-omni-memory.md)

The paired execution tracking for this RFC follows an active layer-boundary
blueprint and ExecPlan, but canonical RFCs do not link hidden workspace paths
directly.

## 3. Problem Statement

Wendao is not only a memory engine. The live system already contains at least
four different classes of assets:

1. runtime cache and replay artifacts
2. episodic and operational memory
3. validated short-term working knowledge
4. durable curated knowledge

Without a formal boundary, the architecture drifts in predictable ways:

1. `Episode` becomes a carrier for non-episodic assets
2. memory lifecycle policy gets reused for durable docs or projected pages
3. cache, strategy, memory, and knowledge trust levels collapse into one
   ambiguous surface

The repository already contains the correct directional clue: memory is
short-term operational state, while long-lived knowledge belongs to a separate
durable retrieval and curation surface.

## 4. Audit Snapshot

### 4.1 `xiuxian-memory-engine` Is an Episodic Core

The current core object is `Episode`, with fields such as:

1. `intent`
2. `experience`
3. `outcome`
4. `q_value`
5. `success_count`
6. `failure_count`
7. `scope`

This is an interaction and experience unit. It is not a durable document or
curated knowledge record.

### 4.2 Retrieval Is Episodic Operational Recall

The current two-phase search implementation explicitly does:

1. semantic recall
2. Q-value reranking

That is a memory-recall design, not a generic durable-knowledge retrieval
contract.

### 4.3 Current Q Semantics Are Utility Estimation

`QTable::update` currently uses:

```text
Q_new = Q_old + alpha * (reward - Q_old)
```

The table stores `discount_factor`, but the update target does not currently
use future-return terms. The accurate description is therefore:

1. MemRL-inspired online utility estimation
2. reward smoothing for episodic reuse
3. not a full temporal-difference future-return learner

### 4.4 Memory Lifecycle Semantics Fit Memory, Not Durable Knowledge

The current lifecycle states are:

1. `Open`
2. `Active`
3. `Cooling`
4. `RevalidatePending`
5. `Purged`
6. `Promoted`

This state machine is suitable for episodic memory. It is not suitable as the
default lifecycle for durable docs, projected pages, or long-lived reference
notes.

### 4.5 Current Naming Drift Must Be Corrected

The code audit also shows several naming or model mismatches that the RFC must
normalize:

1. `mark_accessed()` currently increments `success_count`, which conflates
   access with validated success
2. `update_episode()` rewrites `created_at`, which conflates creation time with
   modification time
3. `store.rs` still opens with LanceDB-oriented wording, while the file itself
   later states direct LanceDB persistence is deferred
4. the current state backends persist episodes, Q-values, and
   `recall_feedback_bias_by_scope`; this is memory state, not a generic
   knowledge registry
5. shared `MemoryGateVerdict::Promote` wording still implies promotion to
   long-term knowledge workflows without naming an explicit target layer

## 5. Canonical Layer Model

### 5.1 Layer A: `CacheArtifact`

Definition:
runtime intermediate artifacts that are not knowledge by default.

Typical objects:

1. plan or blueprint drafts
2. candidate subgraph cache
3. rerank traces
4. solver intermediate results
5. workspace snapshots
6. debate or scratchpad traces

Properties:

1. high temporal sensitivity
2. replayable and invalidatable
3. valid as materialization input
4. not part of the primary user-answer path by default

### 5.2 Layer B: `EpisodicMemory`

Definition:
short-term, retractable, revalidation-aware operational memory.

Ownership:
`xiuxian-memory-engine`

Typical objects:

1. recent workaround experience
2. recent failure recovery experience
3. recent agent interaction traces
4. session- or persona-scoped episodic traces
5. locally useful experience not yet promoted into reusable strategy assets

Properties:

1. semantic recall plus utility reranking
2. reflectable and revalidation-aware
3. purgeable
4. promotable out of episodic memory

### 5.3 Layer C: `WorkingKnowledge`

Definition:
validated short-term knowledge that is more stable than episodic memory but
still reviewable, demotable, and supersedable.

Typical objects:

1. validated `SearchStrategyFlow`
2. currently active blueprint or validated strategy bundles
3. repeated and verified workaround patterns
4. promoted summaries that have reuse value but are not yet durable docs
5. bridge explanations still awaiting durable curation

Properties:

1. reusable and replayable
2. higher trust than episodic memory
3. lower trust than durable knowledge
4. may be superseded, demoted, or promoted onward

### 5.4 Layer D: `DurableKnowledge`

Definition:
long-lived curated knowledge governed by versioning, curation, and verification.

Ownership:
`xiuxian-wendao` and related documentation/projection/graph systems

Typical objects:

1. docs
2. projected pages
3. curated architecture notes
4. reference pages
5. stable glossaries and concept notes
6. long-lived design records

Properties:

1. durable and versioned
2. curated rather than merely accumulated
3. archival and supersession aware
4. never governed by episodic TTL or purge policy

## 6. Ownership Rules

### 6.1 `xiuxian-memory-engine` MUST Own `EpisodicMemory` Only

`xiuxian-memory-engine` owns:

1. episode storage
2. episodic recall
3. utility or Q estimation
4. recall feedback bias
5. episodic lifecycle and memory gate decisions

It must not become:

1. a cache registry
2. a strategy registry
3. a durable docs registry
4. a generic knowledge store

### 6.2 `xiuxian-wendao` MUST Own `DurableKnowledge`

`xiuxian-wendao` owns:

1. durable docs retrieval
2. projected-page retrieval
3. graph-backed durable grounding
4. curated knowledge lifecycle
5. trust-aware fusion with durable grounding as the highest baseline source

### 6.3 `CacheArtifact` and `WorkingKnowledge` SHOULD Live Outside `EpisodeStore`

Recommended default placement:

1. `CacheArtifact` in runtime cache modules
2. `WorkingKnowledge` in runtime strategy or registry modules

Neither should be stored inside `EpisodeStore` by default.

## 7. Allowed and Forbidden Transitions

### 7.1 Allowed by Default

The default allowed paths are:

1. `CacheArtifact -> EpisodicMemory`
2. `CacheArtifact -> WorkingKnowledge`
3. `EpisodicMemory -> WorkingKnowledge`
4. `WorkingKnowledge -> DurableKnowledge`

### 7.2 Forbidden by Default

The default forbidden paths are:

1. `CacheArtifact -> DurableKnowledge`
2. `EpisodicMemory -> DurableKnowledge`
3. `DurableKnowledge -> EpisodicMemory`

Rationale:

1. cache is not knowledge
2. episodic memory is not durable truth
3. durable docs should not be degraded into an episode schema

## 8. Lifecycle Contracts

### 8.1 `CacheArtifact`

Recommended lifecycle:

1. `fresh`
2. `warm`
3. `stale`
4. `invalidated`
5. `archived`

### 8.2 `EpisodicMemory`

Recommended lifecycle:

1. `open`
2. `active`
3. `cooling`
4. `revalidate_pending`
5. `purged`
6. `promoted_out`

`promoted_out` is preferred over plain `promoted` because the target layer must
be named elsewhere instead of remaining implicit.

### 8.3 `WorkingKnowledge`

Recommended lifecycle:

1. `candidate`
2. `active`
3. `reviewed`
4. `cooling`
5. `superseded`
6. `demoted`
7. `promoted_to_durable`

### 8.4 `DurableKnowledge`

Recommended lifecycle:

1. `draft`
2. `published`
3. `verified`
4. `superseded`
5. `archived`

## 9. Retrieval and Trust Policy

### 9.1 `CacheArtifact`

Default use:

1. replay
2. debugging
3. flow reconstruction
4. warm-start assistance
5. explicit operator inspection

It must not enter the primary answer path unless it is explicitly marked as
replay or debug context.

### 9.2 `EpisodicMemory`

Default use:

1. operational recall
2. short-term issue and workaround memory
3. recent execution pattern recall
4. session- or persona-scoped context injection

Default trust:

1. medium and conditional
2. always revalidation-aware
3. never allowed to silently override durable knowledge

### 9.3 `WorkingKnowledge`

Default use:

1. scenario-level reusable flow
2. validated search strategy
3. active workspace heuristics
4. promoted but still reviewable patterns

Default trust:

1. higher than episodic memory
2. lower than durable knowledge
3. explicitly supersedable or demotable

### 9.4 `DurableKnowledge`

Default use:

1. docs retrieval
2. projected-page retrieval
3. stable reference explanation
4. architecture grounding
5. long-term concept grounding

Default trust:

1. highest baseline trust
2. governed by versioning, curation, and verification
3. never routed through episodic TTL or purge policy

## 10. `SearchStrategyFlow` Placement

This RFC defines the default placement as:

> `SearchStrategyFlow` belongs to `WorkingKnowledge` by default.

Rationale:

1. it is not just a runtime cache trace
2. it is not merely one interaction episode
3. before cross-scenario stabilization, it is not yet durable canonical docs

Recommended representation split:

1. flow candidate, branch trace, and debate trace -> `CacheArtifact`
2. validated reusable flow -> `WorkingKnowledge`
3. canonical stable cross-scenario flow -> `DurableKnowledge`

## 11. Required Contract and Model Changes

### 11.1 Narrow Memory Promotion Semantics

Replace ambiguous `Promote` wording with a target-aware meaning such as:

1. `PromoteToWorkingKnowledge`
2. or `PromotedOut` plus a target field

The memory gate must not imply direct durable publication.

### 11.2 Add Explicit Promotion Targets

`MemoryGateDecision` should gain explicit destination metadata such as:

1. `promotion_target`
2. optional `demotion_target`

All promotions are not the same operation.

### 11.3 Split Access from Success

The model should separate:

1. `retrieval_count`
2. `success_count`
3. `failure_count`

Access and validated success are different signals.

### 11.4 Split Creation Time from Update Time

The model should separate:

1. `created_at`
2. `updated_at`
3. `last_accessed_turn` or equivalent access metadata

Age, decay, and update semantics must not share one timestamp.

### 11.5 Clarify Q Semantics

All docs and contracts should say the current implementation is:

1. utility estimation
2. online reward smoothing
3. MemRL-inspired, but not a full TD future-return learner

### 11.6 Add a Separate Durable Promotion Gate

`MemoryGate` should govern only `EpisodicMemory -> WorkingKnowledge`.

`WorkingKnowledge -> DurableKnowledge` requires a separate
`KnowledgePromotionGate` or equivalent durable-governance contract.

## 12. Migration Plan

### Phase 1: Boundary Freeze

Freeze and publish the four-layer vocabulary across docs and architecture
surfaces.

### Phase 2: Memory-Engine Hygiene

Apply minimal corrective changes to the episodic memory engine:

1. split access from success
2. split `created_at` from `updated_at`
3. narrow promotion semantics
4. document memory-only ownership clearly

### Phase 3: Working-Knowledge Registry

Introduce a registry for:

1. `SearchStrategyFlow`
2. promoted workaround patterns
3. active validated strategy bundles

### Phase 4: Durable Knowledge Gate

Add a separate durable-promotion mechanism and stop using `MemoryGate` for
durable publication semantics.

### Phase 5: Layer-Aware Fusion

Make final retrieval fusion explicit:

1. cache for replay and debug
2. memory for operational recall
3. working knowledge for active reusable policy
4. durable knowledge for primary long-term grounding

## 13. Non-Goals

This RFC does not:

1. rewrite all retrieval contracts in one pass
2. define every ontology or strategy schema
3. implement the full working-knowledge registry in this slice
4. convert the memory engine into a full RL system
5. collapse cache, strategy, docs, and memory migration into one change

## 14. Final Decision

The final decision of this RFC is:

1. `xiuxian-memory-engine` is memory-only
2. `EpisodeStore` must not become a generic knowledge store
3. `SearchStrategyFlow` belongs to `WorkingKnowledge` by default
4. durable docs, projected pages, and long-lived notes belong to
   `DurableKnowledge`
5. `MemoryGate` must not directly govern durable promotion or durable purge
