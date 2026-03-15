---
type: feature
metadata:
  title: "Wendao: Multimodal Synaptic Mapping (Visual Semantic Injection)"
  status: "Landed"
  last_updated: "2026-03-12"
---

# Wendao: Multimodal Synaptic Mapping

## 1. Overview

Multimodal Synaptic Mapping enables the [[packages/rust/crates/xiuxian-wendao|Wendao LinkGraph]] to "perceive" and index visual information. By integrating the **Dots OCR** engine from [[packages/rust/crates/xiuxian-llm|xiuxian-llm]], the system can extract semantic annotations from image attachments and establish physical edges between visual data and Markdown knowledge nodes.

## 2. Core Mechanisms

### 2.1 Dots OCR Integration

The system implements a specialized `VisionProvider` that interfaces with the DeepSeek-based OCR runtime.

- **Provider Implementation**: Physically located in [[packages/rust/crates/xiuxian-wendao/src/link_graph/index/build/vision_ingress.rs|vision_ingress.rs]].
- **Process**: Image bytes are preprocessed and fed into `infer_deepseek_ocr_truth` to extract high-confidence text descriptions.

### 2.2 Entity Extraction & Semantic Anchoring

Extracted text is not treated as flat data. The engine uses regex-based extraction to identify code entities:

- **PascalCase Recognition**: Identifies class and module names.
- **Backtick Detection**: Identifies specific identifiers (e.g., `my_function`).
- **Implementation**: Defined in the `extract_entities` operator.

### 2.3 Cross-modal Edge Building

The engine automatically establishes links between images and documents based on content overlap.

- **Mapping Logic**: If an image's OCR description contains terms matching a document's ID or stem, a **Virtual Edge** is established.
- **Operator**: `build_cross_modal_edges` ensures that "searching for text" can now surface "relevant diagrams."

## 3. Physical Architecture

- **Data Model**: [[packages/rust/crates/xiuxian-wendao/src/link_graph/models/attachments.rs|LinkGraphAttachment]] now carries an optional `VisionAnnotation`.
- **Ingress Pipeline**: Integrated into the index build flow via `VisionIngress`.
- **Time-Awareness**: Annotations include a Unix timestamp, enabling integration with [[docs/03_features/wendao-context-snapshot.md|ContextSnap]].

## 4. Related Features

- [[docs/03_features/wendao-living-brain.md|Living Brain (Saliency for Images)]]
- [[docs/03_features/wendao-agentic-retrieval.md|Agentic Retrieval (Visual Reasoning)]]
- [[docs/03_features/wendao-context-snapshot.md|ContextSnap (Memory Anchors)]]

---

_Eyes of the Machine, Wisdom of the Sovereign._
