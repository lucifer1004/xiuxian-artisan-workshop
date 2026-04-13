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
5. `wendao-core-lib` remains a typed Python access layer for Rust-owned
   Flight contracts, not a Python-local search or rerank runtime
6. any future Python-local analyzer implementation should live in a sibling
   package layered on top of `wendao-core-lib`, not inside it

## 2. Alignment

This RFC is governed by:

1. [2026-03-29-python-arrow-flight-boundary-rfc.md](./2026-03-29-python-arrow-flight-boundary-rfc.md)
2. [2026-03-27-wendao-arrow-plugin-flight-rfc.md](./2026-03-27-wendao-arrow-plugin-flight-rfc.md)
3. [2026-03-27-wendao-core-runtime-plugin-migration-rfc.md](./2026-03-27-wendao-core-runtime-plugin-migration-rfc.md)
4. [2026-03-31-python-wendao-analyzer-package-rfc.md](./2026-03-31-python-wendao-analyzer-package-rfc.md)

The paired execution tracking also follows the active core/runtime/plugin
migration blueprint, but canonical RFCs do not link hidden workspace paths
directly.

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
6. keep a clean package boundary between transport substrate and analyzer
   implementation packages in non-Rust ecosystems

## 5. Non-Goals

This RFC does not:

1. reopen Python runtime-surface debates already settled by the boundary RFC
2. justify new Python-local orchestration layers
3. redesign unrelated Rust search subsystems wholesale
4. require every query feature to land in one batch
5. justify moving rerank semantics into Python-local implementation logic
6. define a Python analyzer package as part of this RFC beyond boundary-setting
   and package-role clarity

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
   - rerank top-k response limit
4. rerank exchange response schema
5. real-host behavior proved through `wendao_search_flight_server`

## 7. Rules

All future query-contract work must follow these rules:

1. Rust owns canonical request/response field names and metadata headers
2. Python typed helpers may wrap the contract, but may not invent parallel
   semantics
3. Python support for `repo-search` or `rerank` means typed access to
   Rust-owned Wendao routes, not Python-local execution ownership
4. if Python-local analyzer logic is introduced later, it must live in a
   sibling package that depends on `wendao-core-lib` rather than expanding
   `wendao-core-lib` into an analyzer runtime
5. every new request knob must have a backend-owned rationale
6. every new response field must have a stable producer on the Rust side
7. semantic claims must be proved on the real `xiuxian-wendao` host, not only
   on mock/example servers
8. response fields that are effectively constant, empty, or reconstructible
   from existing stable fields must not be promoted into the contract only for
   surface symmetry
9. when both workspace `wendao.toml` and process env provide rerank-score
   weights on the real `xiuxian-wendao` host path, `wendao.toml` is the
   authoritative source and env remains fallback-only

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

Current repo-search stop-condition guidance:

1. `doc_type` must remain out of the stable repo-search contract while the real
   repo-content host effectively emits only `file`
2. `hierarchical_uri` must remain out of the stable repo-search contract while
   the real repo-content host does not populate it
3. backend-owned evidence should prefer fields that add analyzer value beyond
   what can already be derived from `path`, `tags`, `best_section`, or
   `navigation_*`

Current runtime host settings contract:

1. follow the shared contract in
   [wendao-flight-runtime-host-settings-contract.md](../contracts/wendao-flight-runtime-host-settings-contract.md)
2. precedence claims are not considered landed until a real-host validation
   path proves the conflict behavior end to end
3. new runtime host knobs must satisfy the promotion checklist in that
   contract before they are treated as governed precedence
4. the contract-status table in that contract is normative for deciding
   whether a runtime knob is promoted or still implementation-only
5. host bring-up parameters such as bind address, repo selection, workspace
   root, and bootstrap toggles must remain implementation-only unless they
   gain real shared runtime-policy semantics
6. absent a new knob with real runtime-policy value, the current host-settings
   contract should be treated as temporarily closed rather than expanded for
   symmetry

## 9. Acceptance Criteria

This RFC is considered operational when:

1. active repo-search and rerank evolution cites `Q1` to `Q4`
2. new GTD and ExecPlan entries use query-contract language instead of
   boundary-removal language
3. real-host validation is the default gate for semantic query claims
4. runtime host settings precedence is explicit, documented, and verified on
   the current Flight host paths for the knobs that have been promoted into
   contract status
5. promoted runtime host knobs are registered in
   [wendao-flight-runtime-host-settings-contract.md](../contracts/wendao-flight-runtime-host-settings-contract.md)
6. promoted runtime host knobs satisfy the contract's promotion checklist

The current repo-search response contract is treated as operationally closed
for the real repo-content host when it exposes:

1. identity and rank fields:
   - `doc_id`
   - `path`
   - `title`
   - `score`
   - `language`
2. evidence fields:
   - `best_section`
   - `match_reason`
   - `tags`
3. navigation and structure fields:
   - `navigation_path`
   - `navigation_category`
   - `navigation_line`
   - `navigation_line_end`
   - `hierarchy`

Once that surface is stable on both the mock host and the real
`xiuxian-wendao` host, further repo-search response expansion requires a new
analyzer-facing rationale rather than simple symmetry with broader
`SearchHit` fields.

## 9.1 Current Implementation Baseline

As of 2026-03-31, the current `wendao-core-lib` Arrow Flight baseline is:

1. `repo-search` is operationally supported through typed request helpers and
   typed response rows, with real-host validation on the current
   `xiuxian-wendao` Flight host path
2. `rerank` is operationally supported through typed `do_exchange(...)`
   request/response helpers, shared Rust-owned scoring semantics, and real-host
   validation on both current Flight hosts
3. that `rerank` support is intentionally transport-scoped: Python is exposing
   typed access to a Rust-owned route rather than claiming ownership of rerank
   semantics
4. the intended Python ecosystem split is:
   - `wendao-core-lib` for Arrow/Flight transport and typed contract access
   - a future sibling package such as `xiuxian-wendao-analyzer` for
     Python-local analyzer logic built on top of that transport substrate
5. `top_k` is treated as a closed request-contract slice:
   - omitted by default
   - positive values act as an upper bound
   - blank, zero, and malformed values are rejected on the live Rust host path
6. the current repo-search response contract is treated as closed for this RFC
   unless a new analyzer-facing rationale reopens it
7. further work should prioritize new analyzer-facing query or rerank semantics
   over additional transport-edge hardening on already-closed surfaces

## 10. Open Questions

1. which repo-search semantics deserve first-class request metadata versus
   remaining backend-internal ranking behavior?
2. which repo-search evidence fields beyond `best_section` are stable enough to
   publish without violating the stop-condition rules above?
3. how far should rerank semantics go before a separate rerank-scoring RFC is
   needed?
4. which additional runtime host knobs, if any, deserve promotion into this
   shared precedence contract beyond rerank weights and schema version?

## 11. Decision

Adopt this RFC as the successor planning surface for active Wendao Flight query
contract work.

Treat the current repo-search response contract as operationally sufficient for
the repo-content route, and shift the next semantic-expansion priority toward
rerank/runtime behavior unless a new repo-search field demonstrates concrete
analyzer value on the real host.
