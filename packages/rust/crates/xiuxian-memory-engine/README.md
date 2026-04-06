# xiuxian-memory-engine

Episodic operational memory for the Wendao stack.

## Responsibility

`xiuxian-memory-engine` owns the bounded memory layer only:

- episode storage
- semantic recall plus utility reranking
- Q-value or utility estimation
- recall feedback bias
- episodic lifecycle and gate decisions

This crate does not own durable docs, projected pages, or a generic knowledge
registry.

## Boundary

The formal cross-layer boundary is defined in
[`docs/rfcs/2026-04-05-wendao-memory-layer-boundaries-rfc.md`](../../../../../docs/rfcs/2026-04-05-wendao-memory-layer-boundaries-rfc.md).

Within that model, `xiuxian-memory-engine` is responsible for
`EpisodicMemory`, not for `WorkingKnowledge` or `DurableKnowledge`.

## Current Model

The current crate model is intentionally episodic:

- `Episode` is an interaction or experience unit
- two-phase retrieval is semantic recall followed by Q-value reranking
- `QTable` currently implements online utility smoothing, not full
  temporal-difference future-return learning
- persisted state stores episodes, Q-values, and scope-level recall feedback
  bias

The current hygiene contract also makes these distinctions explicit:

- `retrieval_count` is separate from `success_count` and `failure_count`
- `created_at` is separate from `updated_at`
- memory-gate promotion uses an explicit target layer instead of implying
  direct durable publication

The current host-read-model seam also stays inside episodic ownership:

- `EpisodeStore::memory_projection_rows(...)` exports read-only episode features
- `MemoryProjectionRow` carries scope, embeddings, utility counters, and
  timestamps for Julia compute lanes
- the projection surface does not expose lifecycle mutation or registry writes

## Non-Goals

Do not place the following in this crate:

- durable docs or projected-page ownership
- generic cache-registry behavior
- validated working-knowledge registry behavior
- durable publication or archival policy

## References

- [`docs/01_core/memory/architecture.md`](../../../../../docs/01_core/memory/architecture.md)
- [`packages/rust/crates/xiuxian-memory-engine/src/`](./src/)
