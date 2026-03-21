# Gateway OpenAPI Contract Surface

:PROPERTIES:
:ID: feat-gateway-openapi-contract-surface
:PARENT: [[index]]
:TAGS: feature, gateway, openapi, contracts, qianji
:STATUS: ACTIVE
:VERSION: 1.0
:END:

## Overview

`xiuxian-wendao` now ships one checked-in gateway `OpenAPI` artifact at
`resources/openapi/wendao_gateway.openapi.json` and exposes stable helpers in
`crate::gateway::openapi` so downstream contract lanes can consume the real
gateway surface without regenerating schemas during tests.

This gives `xiuxian-qianji` a file-backed input for `rest_docs` contract
feedback, keeps the runtime route inventory aligned with the bundled document,
and now supports both clean-surface validation and a deterministic persisted
downstream proof.

## Architecture Position

1. Route inventory: `src/gateway/openapi/paths.rs` defines stable route
   constants plus `WENDAO_GATEWAY_ROUTE_CONTRACTS`.
2. Runtime alignment: the gateway router uses those shared path constants
   instead of duplicating literal route strings.
3. Bundled artifact access: `src/gateway/openapi/document.rs` exposes:
   `bundled_wendao_gateway_openapi_document()`,
   `bundled_wendao_gateway_openapi_path()`, and
   `load_bundled_wendao_gateway_openapi_document()`.
4. Clean-surface validation: `xiuxian-qianji` runs
   `run_rest_docs_contract_feedback(...)` against the bundled artifact in
   `tests/integration/test_wendao_live_rest_docs_contract_feedback.rs`.
5. Persisted downstream validation: `xiuxian-qianji` derives a drifted copy of
   the bundled artifact and runs
   `run_and_persist_rest_docs_contract_feedback(...)` in
   `tests/integration/test_wendao_persisted_rest_docs_contract_feedback.rs` so
   Wendao-native entries are actually persisted through a sink.

## Contract Notes

- The bundled artifact is version-controlled and repository-local, so contract
  tests do not depend on runtime schema generation.
- The gateway inventory now also includes `GET /api/repo/sync`,
  `GET /api/repo/overview`, `GET /api/repo/module-search`, and
  `GET /api/repo/symbol-search`, `GET /api/repo/example-search`, and
  `GET /api/repo/doc-coverage`, plus `GET /api/repo/projected-pages`,
  `GET /api/repo/projected-page`, `GET /api/repo/projected-page-index-node`,
  `GET /api/repo/projected-retrieval-hit`,
  `GET /api/repo/projected-retrieval-context`,
  `GET /api/repo/projected-page-family-context`,
  `GET /api/repo/projected-page-family-cluster`,
  `GET /api/repo/projected-page-navigation`,
  `GET /api/repo/projected-page-navigation-search`,
  `GET /api/repo/projected-page-family-search`,
  `GET /api/repo/projected-page-index-tree`,
  `GET /api/repo/projected-page-index-tree-search`,
  `GET /api/repo/projected-page-search`, `GET /api/repo/projected-retrieval`,
  and
  `GET /api/repo/projected-page-index-trees`, which expose the Wendao Repo
  Intelligence source lifecycle, normalized repository overview, normalized
  module lookup, normalized symbol lookup, normalized example lookup,
  documentation coverage, deterministic Stage-2 projected page records,
  deterministic Stage-2 projected page lookup, deterministic Stage-2 projected
  page-index node lookup, deterministic singular mixed Stage-2 hit lookup,
  deterministic singular mixed Stage-2 context lookup, deterministic
  projected-page family context lookup, deterministic singular projected-page
  family cluster lookup, deterministic page-centric projected navigation
  bundle lookup, deterministic projected-page navigation bundle search,
  deterministic projected-page family cluster search,
  deterministic Stage-2 projected page-index tree lookup, deterministic
  Stage-2 projected page-index tree retrieval, deterministic Stage-2 projected
  page retrieval, deterministic mixed Stage-2 retrieval, and deterministic
  builder-native projected page-index trees through the same bundled OpenAPI
  contract surface instead of leaving them CLI-only.
- The repo-intelligence gateway tests now also pin stable bad-request
  contracts for missing `repo`, missing `query`, and invalid repo-sync `mode`,
  so the shared router helper path cannot drift those payloads silently.
- The bundled OpenAPI artifact now also carries static success and bad-request
  examples for the repo-intelligence endpoints, sourced from the same snapshot
  lane that validates the Studio gateway payloads.
- The strict `rest_docs` lane requires non-empty summaries and descriptions,
  success and error response coverage, and request examples for non-trivial
  bodies.
- `POST /api/ui/config` keeps an explicit JSON example in the bundled document,
  and the bundled gateway routes now include documented error responses so the
  real artifact stays clean under `REST-R003`.
- The persisted downstream proof intentionally removes the `POST /api/ui/config`
  example from a temporary artifact copy so `REST-R007` produces one stable
  warning entry that can be persisted end-to-end through the Qianji sink path.

## Validation Targets

- `direnv exec . bash scripts/rust/xiuxian_wendao_live_openapi_contract_feedback.sh`
- `direnv exec . bash scripts/rust/xiuxian_wendao_contract_feedback_consumer.sh`
- `direnv exec . cargo test -p xiuxian-wendao --lib bundled_gateway_openapi_document_`
- `direnv exec . cargo test -p xiuxian-wendao --lib studio_repo_sync_api`
- `direnv exec . cargo test -p xiuxian-qianji --test wendao_live_rest_docs_contract_feedback`
- `direnv exec . cargo test -p xiuxian-qianji --test wendao_persisted_rest_docs_contract_feedback`

:RELATIONS:
:LINKS: [[03_features/203_agentic_navigation]], [[03_features/205_semantic_auditor]], [[03_features/206_openai_semantic_ignition]]
:END:
