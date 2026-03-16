---
id: "20260315152000"
type: knowledge
title: "Feature: Unified Streaming Parser (Transmuter)"
category: "features"
tags:
  - zhenfa
  - streaming
  - zero-copy
  - llm-parsing
  - neuro-symbolic
saliency_base: 9.0
decay_rate: 0.01
metadata:
  title: "Feature: Unified Streaming Parser (Transmuter)"
---

# Feature: Unified Streaming Parser (Transmuter)

## 1. Overview & Goal

The **Unified Streaming Parser** (part of the **Zhenfa Transmutation Layer**) is a high-performance cognitive gateway designed to parse, validate, and display real-time output from multiple LLM CLIs (Claude Code, Gemini CLI, Codex).

**Primary Goal**: To transform raw, non-deterministic CLI streams into structured, verifiable **ZhenfaEvents** with sub-10ms latency and zero memory overhead.

## 2. Key Features

### 2.1. Unified Event Model

Maps heterogeneous provider formats (NDJSON, SSE, OpenAI-JSON) into a single `ZhenfaStreamingEvent` enum.

- **Cognitive Dimensions**: Supports `Thought`, `TextDelta`, `ToolCall`, and `Status` events.
- **Process Supervision**: Categorizes reasoning steps before the final answer is formed.

### 2.2. Symmetrical Zero-Copy Protocol

Every text delta and thought fragment is wrapped in `Arc<str>`.

- **Efficiency**: Eliminates heap allocations during high-frequency token ingestion.
- **Thread-Safety**: Events can be safely shared across concurrent Qianji nodes without cloning.

### 2.3. Incremental Logic Gate (XSD Validation)

Performs "Hot Validation" on partial XML fragments as they stream in.

- **Early-Halt**: Immediately blocks and interrupts agents that violate the `qianji_plan.xsd` schema.
- **Linearity Check**: Enforces sequential step numbering in real-time.

## 3. Architecture (The Include Pattern)

The implementation is split into modular components for hyper-extensibility:

- `mod.rs`: Orchestrator & Provider Detection.
- `traits.rs`: The `StreamingTransmuter` interface.
- `logic_gate.rs`: The incremental XSD validator.
- `claude.rs` / `gemini.rs` / `codex.rs`: Provider-specific parser logic.
- `formatter.rs`: ANSI-aware line-rewriting for terminal display.

## 4. Supported Providers

| Provider        | Format       | Key Events Parsed                       |
| :-------------- | :----------- | :-------------------------------------- |
| **Claude Code** | NDJSON       | `content_block_delta`, `message_stop`   |
| **Gemini CLI**  | Event-Stream | `candidates[].content`, `function_call` |
| **Codex**       | JSON-Chunks  | `choices[].delta`, `finish_reason`      |

---

## Linked Notes

- Parent MOC: [[20260315151000-zhenfa-matrix-moc]]
- Contract System: [[20260315150000-zhenfa-contract-system-spec]]
- Design Blueprint: [[docs/data/blueprints/unified_streaming_parser]]
- Engineering Pattern: [[docs/assets/knowledge/engineering/high-standard-rust-include-pattern]]
