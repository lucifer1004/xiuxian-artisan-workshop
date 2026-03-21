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

:RELATIONS:
:LINKS: [[01_core/101_triple_a_protocol]], [[06_roadmap/401_project_sentinel]], [[06_roadmap/402_repo_intelligence_mvp]], [[06_roadmap/403_document_projection_and_retrieval_enhancement]], [[06_roadmap/404_repo_intelligence_for_sciml_and_msl]], [[06_roadmap/405_large_rust_modularization]]
:END:

---

:FOOTER:
:STANDARDS: v2.0
:LAST_SYNC: 2026-03-21
:END:
