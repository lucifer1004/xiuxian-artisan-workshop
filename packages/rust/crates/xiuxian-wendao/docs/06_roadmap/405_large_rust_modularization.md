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

Priority override:

- `gateway/studio/search/handlers.rs` is the first implementation target and should be split before the numbered waves.

### Wave 1: Contracts and Helpers

- `gateway/studio/types.rs`
- `analyzers/query.rs`
- `analyzers/service/helpers.rs`
- `enhancer/markdown_config.rs`
- `link_graph/addressing/mod.rs`
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
- `link_graph/parser/sections.rs`
- `link_graph/saliency/store/write.rs`
- `ingress/spider.rs`
- `skill_vfs/zhixing/resources.rs`

## Current Status

- [PLANNED] Inventory completed for all 35 oversized files.
- [PLANNED] Each file now has a proposed feature-folder or leaf-module split in the canonical execplan.
- [DONE] `gateway/studio/search/handlers.rs` is split into `gateway/studio/search/handlers/` with interface-only `mod.rs`, preserved public exports, and a green `cargo test -p xiuxian-wendao gateway::studio::search:: --lib` gate.
- [DONE] `gateway/studio/types.rs` is split into `gateway/studio/types/` with interface-only `mod.rs`, grouped DTO leaf modules, preserved public type names, and the same `studio_type_collection()` façade.
- [DONE] `analyzers/query.rs` is split into `analyzers/query/` with interface-only `mod.rs`, query-family leaf modules, and preserved `crate::analyzers::query::*` re-exports.
- [DONE] `search/fuzzy.rs` is split into `search/fuzzy/` with interface-only `mod.rs`, focused helper leaf modules, preserved `search::fuzzy::*` exports, and the crate-visible scoring bridge retained for Tantivy integration.
- [DONE] `search/tantivy.rs` is split into `search/tantivy/` with interface-only `mod.rs`, focused document/index/matcher helper modules, preserved `search::tantivy::*` exports, and unchanged analyzer-facing shared search contracts.
- [DONE] `analyzers/service/mod.rs` is now interface-only, with orchestration logic moved into focused leaf modules while preserving `crate::analyzers::service::*` exports and sibling `super::*` call sites.
- [DONE] The stale tracked `analyzers/service/mod.rs.bak2` monolith is removed after confirming the live `analyzers/service/` leaf modules cover the split, so the service folder no longer carries a shadow copy of the pre-modularization implementation.
- [DONE] `gateway/studio/router/mod.rs` is now interface-only, with Studio state, configured-repository derivation, API error mapping, route assembly, and router-local tests moved into focused leaf modules while preserving `crate::gateway::studio::router::*` exports and the existing `code_ast`, `config`, `handlers`, and `sanitization` child modules.
- [DONE] `cargo check -p xiuxian-wendao --lib --keep-going` remains green in the current worktree after the first six modularization slices.
- [DONE] `cargo test -p xiuxian-wendao analyzers::service:: --lib`, `cargo test -p xiuxian-wendao repo_sync_endpoint_returns_repo_status_payload --lib`, and `cargo test -p xiuxian-wendao --bin wendao test_build_plugin_registry_bootstraps_builtin_plugins` are green after the `analyzers/service/mod.rs` split.
- [DONE] `cargo check -p xiuxian-wendao`, `cargo test -p xiuxian-wendao gateway::studio::router:: --lib`, `cargo test -p xiuxian-wendao --lib studio_repo_sync_api -- --nocapture`, and `cargo test -p xiuxian-wendao --bin wendao test_gateway_server_bind -- --nocapture` are green after the `gateway/studio/router/mod.rs` split.
- [DONE] `gateway/studio/repo_index/state.rs` is now a feature folder with interface-only `state/mod.rs`, while task-queue control, coordination logic, fingerprinting, code-document collection, status filtering, language inference, and repo-index tests live in focused leaf modules without changing the `RepoIndexCoordinator` surface.
- [DONE] `cargo fmt -p xiuxian-wendao`, `cargo check -p xiuxian-wendao --lib --keep-going`, `cargo test -p xiuxian-wendao gateway::studio::repo_index:: --lib`, `cargo test -p xiuxian-wendao gateway::studio::search:: --lib`, and `cargo test -p xiuxian-wendao repo_sync_endpoint_returns_repo_status_payload --lib` are green after the `gateway/studio/repo_index/state.rs` split.
- [DONE] `gateway/studio/router/handlers/repo.rs` is now a feature folder with interface-only `repo/mod.rs`, while `query.rs`, `parse.rs`, `shared.rs`, `analysis.rs`, `index.rs`, `pages.rs`, `retrieval.rs`, `family.rs`, and `refine.rs` carry the handler families without changing the existing repo handler names.
- [DONE] `cargo fmt -p xiuxian-wendao`, `cargo check -p xiuxian-wendao --lib --keep-going`, `cargo test -p xiuxian-wendao gateway::studio::studio_repo_sync_api_tests:: --lib`, `cargo test -p xiuxian-wendao gateway::studio::router:: --lib`, and `cargo test -p xiuxian-wendao gateway::studio::repo_index:: --lib` are green after the `gateway/studio/router/handlers/repo.rs` split.
- [DONE] `gateway/studio/vfs.rs` is now a feature folder with interface-only `vfs/mod.rs`, while `filters.rs`, `roots.rs`, `scan.rs`, `navigation.rs`, `content.rs`, `categories.rs`, and `tests.rs` carry VFS scanning, root resolution, navigation, and content reads without changing the active Studio VFS entry points.
- [DONE] `cargo fmt -p xiuxian-wendao`, `cargo check -p xiuxian-wendao --lib --keep-going`, and `cargo test -p xiuxian-wendao gateway::studio:: --lib` are green after the `gateway/studio/vfs.rs` split.
- [DONE] `gateway/openapi/paths.rs` is now a feature folder with interface-only `paths/mod.rs`, while `analysis.rs`, `graph.rs`, `repo.rs`, `search.rs`, `shared.rs`, `ui.rs`, and `vfs.rs` carry the route constants and route inventory without changing `crate::gateway::openapi::paths::*`, `RouteContract`, or `WENDAO_GATEWAY_ROUTE_CONTRACTS`.
- [DONE] Public constant-surface parity against the pre-split index version, route-inventory count parity, file-local `rustfmt`, standalone `rustc` re-export compilation, and `git diff --check` are green for the `gateway/openapi/paths.rs` slice.
- [DONE] `gateway/studio/search/source_index.rs` is now a feature folder with interface-only `source_index/mod.rs`, while `ast.rs`, `symbols.rs`, `markdown.rs`, `navigation.rs`, and `filters.rs` carry the index build, markdown-hit synthesis, navigation payload assembly, and scan filtering logic without changing the existing `build_ast_index` / `build_symbol_index` call path.
- [DONE] `cargo fmt -p xiuxian-wendao`, `cargo check -p xiuxian-wendao --lib --keep-going`, `cargo test -p xiuxian-wendao gateway::studio::search:: --lib`, and `git diff --check` are green after the `gateway/studio/search/source_index.rs` split.
- [DONE] `analyzers/service/search.rs` is now a feature folder with interface-only `search/mod.rs`, while `documents.rs`, `legacy.rs`, `indexed_exact.rs`, `indexed_fuzzy.rs`, `ranking.rs`, `overview.rs`, `module.rs`, `symbol.rs`, `example.rs`, `imports.rs`, `coverage.rs`, and `tests.rs` carry the repo overview, ranked search, import search, and doc-coverage logic without changing the existing `crate::analyzers::service::*` search APIs.
- [DONE] `analyzers/service/projection.rs` is now a feature folder with interface-only `projection/mod.rs`, while `gap.rs`, `pages.rs`, `page.rs`, `family.rs`, `navigation.rs`, `search.rs`, `index_tree.rs`, `retrieval.rs`, `registry.rs`, and `tests.rs` carry the projected-page, family, navigation, tree, retrieval, and gap-report wrappers without changing the existing `crate::analyzers::service::*` projection APIs.
- [DONE] `analyzers/service/helpers.rs` is now a feature folder with interface-only `helpers/mod.rs`, while `uri.rs`, `ecosystem.rs`, `path.rs`, `ranking.rs`, `backlinks.rs`, `projection_lookup.rs`, `example_relations.rs`, `scope.rs`, and `tests.rs` carry the URI, ecosystem, path, ranking, backlink, projection lookup, example relation, and scope helpers without changing the existing analyzer-service call sites.
- [DONE] `enhancer/markdown_config.rs` is now a feature folder with interface-only `markdown_config/mod.rs`, while `types.rs`, `index.rs`, `blocks.rs`, `links.rs`, and `tests.rs` carry markdown config block parsing, link-target extraction, and memory-index helpers without changing the `resource_registry` and enhancer API surface.
- [DONE] `skill_vfs/internal_manifest.rs` is now a feature folder with interface-only `internal_manifest/mod.rs`, while `types.rs`, `load.rs`, `authority.rs`, and `tests.rs` carry internal skill manifest parsing, authority resolution, and workflow-type helpers without changing `skill_vfs::internal_manifest::*`.
- [DONE] `link_graph/addressing/mod.rs` is now a feature folder with interface-only `mod.rs`, while `address.rs`, `resolve.rs`, `indices.rs`, `node_lookup.rs`, `mutation.rs`, `errors.rs`, and `tests.rs` preserve the registry, topology, skeleton-rerank, and structural-transaction child modules.
- [DONE] `link_graph/index.rs` is now a feature folder with interface-only `mod.rs`, while `constants.rs`, `types.rs`, `lookup.rs`, `symbol_cache.rs`, and the preserved `build/`, `search/`, `ids.rs`, `page_indices.rs`, `passages.rs`, `ppr.rs`, `rank.rs`, `scoring.rs`, `semantic_documents.rs`, `shared.rs`, and `traversal.rs` child namespaces keep the public `link_graph::index::*` surface stable.
- [DONE] `link_graph/parser/code_observation.rs` is now a feature folder with interface-only `mod.rs`, while `types.rs`, `glob.rs`, `extract.rs`, `format.rs`, and the preserved tests fixture carry code-observation parsing, glob matching, formatting, and extraction logic without changing `link_graph::parser::code_observation::*`.
- [DONE] `cargo fmt -p xiuxian-wendao`, `cargo check -p xiuxian-wendao --lib --keep-going`, `cargo test -p xiuxian-wendao skill_vfs::internal_manifest::tests:: --lib`, `cargo test -p xiuxian-wendao link_graph::addressing:: --lib`, `cargo test -p xiuxian-wendao link_graph::index:: --lib`, `cargo test -p xiuxian-wendao link_graph::parser::code_observation:: --lib`, and `git diff --check` are green after the `skill_vfs/internal_manifest.rs`, `link_graph/addressing/mod.rs`, `link_graph/index.rs`, and `link_graph/parser/code_observation.rs` splits.
- [IN-PROGRESS] `link_graph/parser/sections.rs` is the next bounded facade slice under the active execplan.

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
:LAST_SYNC: 2026-03-22
:END:
