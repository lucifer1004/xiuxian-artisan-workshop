# Large Rust File Modularization

:PROPERTIES:
:ID: wendao-large-rust-modularization
:PARENT: [[index]]
:TAGS: roadmap, refactor, modularization, rust, gateway, link-graph
:STATUS: PLANNED
:END:

## Mission

This roadmap note tracks a lossless modularization pass over oversized Rust source files in `xiuxian-wendao`.

Inventory date: `2026-03-21`

Inventory scope:

- `packages/rust/crates/xiuxian-wendao/src/**/*.rs`
- threshold: files larger than 400 lines
- current count: 35 files

The canonical per-file execution plan lives in `[[.cache/codex/execplans/wendao-large-rust-modularization.md]]`.

## Why This Slice Exists

- Several files are serving as DTO warehouses or transport façades and have accumulated unrelated responsibilities.
- `gateway/studio/router/mod.rs` and `analyzers/service/mod.rs` currently violate the repository rule that `mod.rs` should be interface-only.
- `semantic_check`, `sentinel`, and `link_graph` now have enough internal phases that file-level boundaries hide the real ownership model.
- A lossless split is a prerequisite for later behavior work because it reduces blast radius and makes targeted testing feasible.

## Delivery Waves

### Wave 1: Contracts and Helpers

- `gateway/studio/types.rs`
- `analyzers/query.rs`
- `analyzers/service/helpers.rs`
- `enhancer/markdown_config.rs`
- `skill_vfs/internal_manifest.rs`
- `link_graph/models/records/markdown_block.rs`

### Wave 2: Search and Projection

- `search/fuzzy.rs`
- `search/tantivy.rs`
- `analyzers/projection/builder.rs`
- `analyzers/projection/search.rs`
- `analyzers/service/search.rs`
- `analyzers/service/projection.rs`
- `gateway/studio/search/source_index.rs`

### Wave 3: Gateway and Router

- `gateway/studio/search/handlers.rs`
- `gateway/studio/router/handlers/repo.rs`
- `gateway/studio/router/mod.rs`
- `gateway/studio/vfs.rs`
- `gateway/studio/repo_index/state.rs`
- `gateway/openapi/paths.rs`

### Wave 4: Governance and Audit

- `zhenfa_router/native.rs`
- `zhenfa_router/native/sentinel.rs`
- `zhenfa_router/native/semantic_check.rs`
- `zhenfa_router/native/semantic_check/docs_governance/collection.rs`
- `zhenfa_router/native/semantic_check/docs_governance/tests.rs`
- `zhenfa_router/native/audit/audit_bridge.rs`
- `zhenfa_router/native/audit/fuzzy_suggest.rs`
- `zhenfa_router/native/audit/fix.rs`

### Wave 5: Link-Graph and Remaining Infrastructure

- `link_graph/addressing/mod.rs`
- `link_graph/index.rs`
- `link_graph/parser/code_observation.rs`
- `link_graph/parser/sections.rs`
- `link_graph/saliency/store/write.rs`
- `ingress/spider.rs`
- `skill_vfs/zhixing/resources.rs`

## Current Status

- [PLANNED] Inventory completed for all 35 oversized files.
- [PLANNED] Each file now has a proposed feature-folder or leaf-module split in the canonical execplan.
- [NOT STARTED] No Rust module paths have been moved yet.

## Local Constraints

- The governing execution-plan policy lives at `.agent/PLANS.md`.
- The workspace does not currently contain `.data/blueprints/project_anchor_semantic_addressing.md`.
- Because the active blueprint file is absent, this roadmap treats current code topology and public API paths as the operative contract for the refactor.

:RELATIONS:
:LINKS: [[index]], [[06_roadmap/402_repo_intelligence_mvp]], [[06_roadmap/403_document_projection_and_retrieval_enhancement]], [[06_roadmap/404_repo_intelligence_for_sciml_and_msl]], [[.cache/codex/execplans/wendao-large-rust-modularization.md]]
:END:

---

:FOOTER:
:STANDARDS: v2.0
:LAST_SYNC: 2026-03-21
:END:
