# Wendao DocOS Kernel: Map of Content

:PROPERTIES:
:ID: wendao-moc
:TYPE: INDEX
:STATUS: ACTIVE
:END:

Standardized documentation repository for the Wendao DocOS Kernel, leveraging AST-based identity and structured properties.

## 📁 01_core: Architecture & Foundation

:PROPERTIES:
:ID: core-foundation
:OBSERVE: lang:rust "pub enum ThisDoesNotExistAnywhere { $$$ }"
:CONTRACT: must_contain("Id", "Path", "Hash")
:END:

- [[01_core/101_triple_a_protocol]]: Identity-based addressing.
- [[01_core/102_atomic_mutation]]: Byte-level modification safety.
- [[01_core/103_package_layering]]: Package ownership rules for `core`, `runtime`, `wendao`, and plugin crates.

## 📁 02_parser: Parser Architecture

- [02_parser/index](02_parser/index.md): Canonical parser namespace, parser-family matrix, and parser-vs-helper rules.

## 📁 03_features: Functional Ledger

:PROPERTIES:
:ID: functional-ledger
:OBSERVE: lang:rust "pub struct LinkGraphIndex { $$$ }"
:END:

- [[03_features/201_property_drawers]]: Metadata management.
- [[03_features/202_block_addressing]]: Paragraph-level granularity.
- [[03_features/203_agentic_navigation]]: Reasoning-driven discovery.
- [[03_features/204_code_observation]]: Non-invasive sgrep binding.
- [[03_features/205_semantic_auditor]]: Native sentinel engine.
- [[03_features/206_openai_semantic_ignition]]: OpenAI-compatible query ignition bridge.
- [[03_features/207_gateway_openapi_contract_surface]]: Stable gateway OpenAPI contract surface for `rest_docs`.
- [[03_features/208_performance_gate_v1]]: Feature-gated Wendao performance gate, stress lane, and Criterion analysis layer.
- [[03_features/209_datafusion_sql_query_surface]]: Request-scoped DataFusion SQL query surface, discovery catalogs, and snapshot contract.
- [[03_features/210_search_queries_architecture]]: Native Flight plus one shared queries system for SQL, FlightSQL, GraphQL, REST, and CLI query entrypoints.
- [[03_features/211_graphql_query_surface]]: First DataFusion-aligned GraphQL table-query adapter over the shared SQL surface.
- [[03_features/212_flightsql_query_surface]]: First FlightSQL statement-query and sql-info adapter over the shared SQL surface.
- [[03_features/213_rest_query_surface]]: First thin REST-style request/response adapter over the shared query service.

## 📁 05_research: Theoretical Hardening

- [[05_research/301_research_papers]]: Academic foundations.

## 📁 06_roadmap: Future Evolution

:PROPERTIES:
:ID: roadmap-sentinel
:OBSERVE: lang:rust "pub trait AuditBridge { $$$ }"
:CONTRACT: must_contain("generate_fixes", "apply_fixes")
:END:

- [[06_roadmap/401_project_sentinel]]: Project Sentinel (Auditing).
- [[06_roadmap/402_repo_intelligence_mvp]]: Repo Intelligence common core and plugin API MVP.
- [[06_roadmap/403_document_projection_and_retrieval_enhancement]]: Document projection and retrieval enhancement on top of Repo Intelligence.
- [[06_roadmap/404_repo_intelligence_for_sciml_and_msl]]: SciML and MSL repo intelligence architecture and boundary mapping.
- [[06_roadmap/405_large_rust_modularization]]: Lossless modularization plan for oversized Rust files in `xiuxian-wendao`.
- [[06_roadmap/409_core_runtime_plugin_surface_inventory]]: `P0 / Mapping Gate` inventory for Julia-specific host surfaces and their target `core` / `runtime` / plugin-package ownership.
- [[06_roadmap/410_p1_generic_plugin_contract_staging]]: `P1` staging note for generic plugin capability, artifact, provider, and transport contracts.
- [[06_roadmap/411_p1_first_code_slice_plan]]: First `P1` implementation slice plan with module tree, compatibility shims, and file touch order.
- [[06_roadmap/412_core_runtime_plugin_program]]: Program-level execution entrypoint for the overall core/runtime/plugin migration.
- [[06_roadmap/413_m2_core_extraction_package_list]]: First package list for the physical `xiuxian-wendao-core` extraction.
- [[06_roadmap/414_m3_runtime_extraction_package_list]]: First package list for the physical `xiuxian-wendao-runtime` extraction.
- [[06_roadmap/415_m4_julia_externalization_package_list]]: First package list for Julia ownership externalization into `xiuxian-wendao-julia`.
- [[06_roadmap/416_compatibility_retirement_ledger]]: Program ledger for compatibility surface retirement order, unlock phases, and target end states.
- [[06_roadmap/417_wendao_package_boundary_matrix]]: Contributor-facing boundary matrix for `xiuxian-wendao-core`, `xiuxian-wendao-runtime`, and `xiuxian-wendao`.
- [[06_roadmap/418_julia_plugin_first_rollout]]: Julia-first plugin rollout for keeping thick Julia implementation inside `xiuxian-wendao-julia`.
- `src/compatibility/`: Explicit crate-root compatibility namespace for compat-first and legacy Julia migration paths.
- `docs/rfcs/2026-03-27-wendao-arrow-plugin-flight-rfc.md`: Arrow-first plugin protocol with Flight-first transport and Arrow IPC fallback.
- `docs/rfcs/2026-03-27-wendao-core-runtime-plugin-migration-rfc.md`: Complete migration path from monolithic Wendao ownership toward `core`, `runtime`, and independently published plugin packages.

Transient blueprint and ExecPlan tracking records are intentionally omitted
from this canonical index. Use the RFC and roadmap notes above as the stable
documentation surface.

:RELATIONS:
:LINKS: [[01_core/101_triple_a_protocol]], [[06_roadmap/401_project_sentinel]], [[06_roadmap/402_repo_intelligence_mvp]], [[06_roadmap/403_document_projection_and_retrieval_enhancement]], [[06_roadmap/404_repo_intelligence_for_sciml_and_msl]], [[06_roadmap/405_large_rust_modularization]], [[06_roadmap/409_core_runtime_plugin_surface_inventory]], [[06_roadmap/410_p1_generic_plugin_contract_staging]], [[06_roadmap/411_p1_first_code_slice_plan]]
:END:

---

:FOOTER:
:STANDARDS: v2.0
:LAST_SYNC: 2026-04-04
:END:
