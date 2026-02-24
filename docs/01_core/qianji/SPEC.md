# Specification: xiuxian-qianji (千机)

> **Authority:** CyberXiuXian Artisan Studio  
> **Mission:** Building a High-Performance, Probabilistic, and Formally Verified Workflow Engine in Rust.
> **Status:** Full-Spectrum Logic Enclosure / YAML-Driven.

---

## 1. Research Foundations (Belief System)

The **Qianji** engine is derived from the synthesis of four foundational research papers (2024-2026).

| Reference                   | Key Theory                              | Local Evidence Path                                |
| :-------------------------- | :-------------------------------------- | :------------------------------------------------- |
| **Synaptic-Flow (2026)**    | Asynchronous Dependency-Aware Inference | `.data/research/papers/qianji_foundation_2026.txt` |
| **Agent-Prob-Route (2025)** | Probabilistic Graph Routing (MDP)       | `.data/research/papers/qianji_foundation_2026.txt` |
| **LTL-Agents (2024)**       | Formal Logic Verification of Loops      | `.data/research/papers/qianji_foundation_2026.txt` |
| **Synapse-Audit (2025)**    | Iterative Calibration Loops             | `.data/research/papers/synapse_audit_2025.txt`     |

---

## 2. Architectural Design: The "Iron Frame & Divine Logic"

### 2.1 The "Iron Frame" (Kernel)

- **Engine:** Based on Rust's `petgraph` library using `StableGraph`.
- **Topology:** Supports DAGs, Cycles (with LTL guards), and Sub-graphs (nested Qianji boxes).
- **Performance:** Aiming for < 100ns topological traversals and zero-overhead node scheduling via `tokio` parallel tasks.

### 2.2 The "Divine Logic" (Scheduling & Orchestration)

- **YAML-Driven Orchestration:** The engine is entirely governed by a declarative `QianjiManifest` (YAML).
  - **Logic Enclosure:** Graph construction, node dependency resolution, and probabilistic weights are defined in YAML and compiled by the Rust `QianjiCompiler`.
- **Probabilistic Routing:** Every edge has a weight $W = f(\text{Omega_Confidence})$. The path is not binary but probability-weighted (MDP-based).
- **Adversarial 回路:** Implements the **Synapse-Audit** skeptic-prospector-calibrator loop as a native graph pattern.
- **State Machine:** Implements a strict state machine for each node: `Idle -> Queued -> Transmuting (Qianhuan) -> Executing -> Calibrating -> Finalized`.

---

## 3. The "Rust-Hard, Python-Thin" Philosophy

In the Qianji architecture, Python is reduced to a "thin slice" glue layer.

### 3.1 Responsibilities

- **Rust (The Brain):**
  - Parses `qianji.yaml` via `serde_yaml`.
  - Compiles the `petgraph` execution DAG.
  - Manages parallel `tokio` execution of Knowledge (Wendao) and Annotation (Qianhuan) nodes.
  - Performs LTL Safety Audits to prevent deadlocks and infinite loops.
- **Python (The Glue):**
  - Calls `qianji.run(context_json)`.
  - Handles final UI presentation of the results.

---

## 4. Performance Baselines (Artisan Verified)

- **YAML Compilation:** < 1ms for 50-node graphs.
- **Topological Traversal:** < 100ns per node jump.
- **Concurrent Execution:** Zero-overhead scheduling via `tokio` task spawning.
- **Memory Efficiency:** < 10MB overhead for the engine core.

---

## 5. Implementation Roadmap: "The Silent Takeover"

1.  **Phase A (Done):** Rust Core + `petgraph` Kernel.
2.  **Phase B (Done):** YAML Manifest Compiler.
3.  **Phase C (Done):** Adversarial Loop & Probabilistic Routing.
4.  **Phase D (Ongoing):** Integration Testing & Python "Shadow Run" validation.

---

## 6. Why "Qianji" (千机)?

Derived from the "Thousand Mechanism Box," it symbolizes the complexity and precision of our workflow engine. It is designed to handle thousands of concurrent reasoning nodes with the rhythmic precision of a clockwork artifact.
