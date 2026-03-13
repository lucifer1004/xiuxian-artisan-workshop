---
type: feature
metadata:
  title: "Wendao: The Living Brain (Hebbian Neural Evolution)"
  status: "Landed"
  last_updated: "2026-03-12"
---

# Wendao: The Living Brain (LivingBrain 3.0)

## 1. Overview

The "Living Brain" feature implements biological-inspired neural evolution within the [[packages/rust/crates/xiuxian-wendao|Wendao LinkGraph]]. It allows knowledge nodes and edges to evolve their structural authority based on retrieval frequency and temporal decay, following the principles of **[[.data/research/papers/HippoRAG_2405.14831.pdf|HippoRAG 2 (2025)]]**.

## 2. Core Mechanisms

### 2.1 Hebbian Saliency Evolution

Nodes gain "Saliency" (显著性) through retrieval hits. The evolution logic is physically anchored in [[packages/rust/crates/xiuxian-wendao/src/link_graph/saliency/calc.rs|saliency/calc.rs]].

- **Decay Formula**: $ S*{new} = \text{clamp}(S*{base} \cdot e^{-\lambda \Delta t} + \alpha \cdot \ln(1 + \text{activations}), [min, max]) $
- **Karmic Link**: This signal provides the "Global Temperature" for [[docs/03_features/wendao-agentic-retrieval.md|Agentic Retrieval]].

### 2.2 Tiered Co-activation (协同放电)

When a node $A$ is retrieved, its structural neighbors $B_i$ are also "touched". This is managed by the [[packages/rust/crates/xiuxian-wendao/src/link_graph/saliency/touch.rs|Async Saliency Worker]].

- **Signal Spread**: $\Delta \text{activation}_{B_i} = \frac{1.0}{\text{rank}_i + 1}$

### 2.3 Synaptic Plasticity (突触可塑性)

Reinforces the edges in [[packages/rust/crates/xiuxian-wendao/src/link_graph/saliency/store/write.rs|Valkey Persistence]], directly affecting [[docs/03_features/wendao-context-snapshot.md|ContextSnap]] scoring.

## 3. Physical Architecture

- **Kernel Hub**: [[packages/rust/crates/xiuxian-wendao/src/link_graph/saliency/mod.rs|saliency/mod.rs]]
- **Worker Queue**: `mpsc::sync_channel` with `OnceLock` singleton.

## 4. Related Features

- [[docs/03_features/wendao-agentic-retrieval.md|Agentic Retrieval]]
- [[docs/03_features/wendao-context-snapshot.md|ContextSnap]]
- [[AGENTS.md|Incremental Evolution Protocol]]
