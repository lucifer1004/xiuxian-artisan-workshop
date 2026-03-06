# Audit & Blueprint Tracker (神识追踪仪)

This document tracks the alignment between logical blueprints and physical implementation.

## 1. High-Priority Audit Queue

| Task ID  | Component     | Blueprint                      | Target Path                                        | Status                                 |
| :------- | :------------ | :----------------------------- | :------------------------------------------------- | :------------------------------------- | ------ |
| **14.8** | Vision Core   | Dots OCR Integration Blueprint | `xiuxian-llm/src/llm/vision/*`                     | [READY]                                |
| **15.2** | Web Engine    | [[Spider-Native Integration    | .data/blueprints/spider_native_integration.md]]    | `xiuxian-llm/src/web/spider.rs`        | [TODO] |
| **15.3** | Wendao Bridge | [[Spider-to-Wendao Bridge      | .data/blueprints/spider_wendao_bridge.md]]         | `xiuxian-wendao/src/ingress/spider.rs` | [TODO] |
| **16.1** | Memory Core   | [[Memory Matrix Unification    | .data/blueprints/memory_matrix_unification.md]]    | `xiuxian-memory-engine/src/matrix.rs`  | [TODO] |
| **16.2** | Archiver      | [[Auto-Archiving Logic         | .data/blueprints/auto_archiving_logic.md]]         | `xiuxian-qianji/src/swarm/archive.rs`  | [TODO] |
| **16.3** | Hybrid Search | [[Cognitive Recall Integration | .data/blueprints/cognitive_recall_integration.md]] | `xiuxian-wendao/src/search/hybrid.rs`  | [TODO] |

## 2. Global Compliance Standards

- [TIER-3] Gate audit required before marking any task as DONE.
- [ZERO-COPY] String cloning in hot paths is a BLOCKER.
- [SSoT] All configs must derive from `xiuxian.toml`.
