---
type: knowledge
title: "RFC: Wendao Flight Query Contract Evolution"
category: "rfc"
status: "draft"
authors:
  - codex
created: 2026-03-31
tags:
  - rfc
  - wendao
  - arrow
  - flight
  - query
  - contract
metadata:
  title: "RFC: Wendao Flight Query Contract Evolution"
---

# RFC: Wendao Flight Query Contract Evolution

## 1. Summary

This RFC defines the post-boundary work for Wendao Flight query evolution.

The decision is:

1. the Python boundary RFC remains the completed baseline
2. active work now shifts to stable query-contract design across Python and Rust
3. repo-search and rerank must evolve through explicit request/response
   contracts instead of ad hoc helper growth
4. real-host validation against `xiuxian-wendao` is required before semantics
   are treated as landed

## 2. Alignment

This RFC is governed by:

1. [2026-03-29-python-arrow-flight-boundary-rfc.md](./2026-03-29-python-arrow-flight-boundary-rfc.md)
2. [2026-03-27-wendao-arrow-plugin-flight-rfc.md](./2026-03-27-wendao-arrow-plugin-flight-rfc.md)
3. [2026-03-27-wendao-core-runtime-plugin-migration-rfc.md](./2026-03-27-wendao-core-runtime-plugin-migration-rfc.md)
4. [wendao_arrow_plugin_core_runtime_migration.md](../../.data/blueprints/wendao_arrow_plugin_core_runtime_migration.md)

## 3. Problem Statement

The repository has already completed most of the Python-boundary collapse, but
active work has moved into a different problem:

1. repo-search request metadata is growing
2. rerank request/response contracts are becoming typed
3. real-host query semantics now matter as much as transport shape
4. those changes no longer fit the purpose of the boundary/removal RFC

Without a successor RFC, the query-contract line will drift inside a closure
document whose main job was to delete historical Python runtime surface.

## 4. Goals

This RFC has the following goals:

1. define the stable Wendao Flight query surfaces that may evolve
2. require explicit Rust-owned request and response contracts
3. require Python typed helpers to stay thin and contract-driven
4. require real-host validation against `xiuxian-wendao` for semantic claims
5. separate transport-contract evolution from historical boundary cleanup

## 5. Non-Goals

This RFC does not:

1. reopen Python runtime-surface debates already settled by the boundary RFC
2. justify new Python-local orchestration layers
3. redesign unrelated Rust search subsystems wholesale
4. require every query feature to land in one batch

## 6. Contract Areas

The active query-contract surface currently includes:

1. repo-search request metadata
   - query text
   - limit
   - language filters
   - path-prefix filters
   - title filters
   - tag filters
   - filename filters
2. repo-search response columns
   - `doc_id`
   - `path`
   - `title`
   - `best_section`
   - `score`
   - `language`
3. rerank exchange request schema
4. rerank exchange response schema
5. real-host behavior proved through `wendao_search_flight_server`

## 7. Rules

All future query-contract work must follow these rules:

1. Rust owns canonical request/response field names and metadata headers
2. Python typed helpers may wrap the contract, but may not invent parallel
   semantics
3. every new request knob must have a backend-owned rationale
4. every new response field must have a stable producer on the Rust side
5. semantic claims must be proved on the real `xiuxian-wendao` host, not only
   on mock/example servers

## 8. Workstreams

### Q1: Request Contract Stabilization

Goal:

1. finish stabilizing meaningful repo-search and rerank request inputs
2. reject blank or malformed metadata consistently across Rust and Python

### Q2: Response Contract Stabilization

Goal:

1. publish and validate stable response columns for repo-search and rerank
2. expose those fields as typed Python rows without extra parsing layers

### Q3: Real-Host Semantic Validation

Goal:

1. prove backend filtering, ranking, and evidence semantics on the real
   `xiuxian-wendao` host
2. stop relying on transport-only or mock-only confidence

### Q4: Stop Conditions

Goal:

1. define what counts as sufficient contract closure for the current route set
2. prevent unbounded accretion of query-specific knobs without RFC review

## 9. Acceptance Criteria

This RFC is considered operational when:

1. active repo-search and rerank evolution cites `Q1` to `Q4`
2. new GTD and ExecPlan entries use query-contract language instead of
   boundary-removal language
3. real-host validation is the default gate for semantic query claims

## 10. Open Questions

1. which repo-search semantics deserve first-class request metadata versus
   remaining backend-internal ranking behavior?
2. which repo-search evidence fields beyond `best_section` are stable enough to
   publish?
3. how far should rerank semantics go before a separate rerank-scoring RFC is
   needed?

## 11. Decision

Adopt this RFC as the successor planning surface for active Wendao Flight query
contract work.
