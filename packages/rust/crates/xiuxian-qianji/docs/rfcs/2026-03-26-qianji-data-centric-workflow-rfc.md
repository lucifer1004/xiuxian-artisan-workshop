---
type: knowledge
title: "RFC: Data-Centric Workflow Orchestration on Wendao Relations"
category: "rfc"
status: "draft"
authors:
  - codex
created: 2026-03-26
tags:
  - rfc
  - qianji
  - wendao
  - datafusion
  - arrow
  - multi-agent
  - workflow
---

# RFC: Data-Centric Workflow Orchestration on Wendao Relations

## 1. Summary

This RFC proposes a paradigm shift for the **Qianji Workflow Engine**: evolving from a traditional task-based trigger system to a **Data-Centric Orchestration** model built on top of the Wendao Query Engine. Qianji will treat workflows as orchestration plans over Wendao-produced relations where data is passed between agents as zero-copy Arrow `RecordBatches`.

Qianji does **not** own query planning, retrieval planning, graph planning, or storage policy. Those concerns belong to Wendao. Qianji owns workflow orchestration, agent scheduling, audit and repair loops, and relation handoff between workflow stages.

## 2. Motivation

The current Qianji architecture relies on serializing agent outputs (often JSON) and passing them through the Zhenfa signal bus. While robust, this approach encounters significant bottlenecks when:

1. **Large Contexts**: Passing large AST structures or reference sets between agents incurs heavy CPU and memory overhead.
2. **Tight Loops**: Multi-agent loops (e.g., iterative document fixing) suffer from repeated serialization/deserialization cycles.
3. **Black-box Reasoning**: Users cannot see the "data funnel" as an agent filters millions of rows down to a few insights.

By embracing a Data-Centric model on top of Wendao relations, we can achieve **sub-100ms multi-agent handoffs** even with GB-scale datasets while keeping query semantics centralized in Wendao.

## 3. Core Architectural Changes

### 3.1 Zero-Copy Agent Handoff (Arrow Handoff)

Qianji will standardize on **Apache Arrow 58** as the inter-agent memory contract.

- **Mechanism**: Instead of piping JSON strings, Qianji passes `Arc<RecordBatch>` handles.
- **Benefit**: An "AST Extraction Agent" defined in Qianhuan can produce a batch of 50,000 nodes, and a "Diagnostic Agent" can consume it instantly without touching the heap.

### 3.2 Workflow-as-an-Orchestration-Plan

Workflows are no longer static. They become orchestration plans over Wendao relations and Wendao operator outputs.

- **Dynamic Partitioning**: Qianji consumes Wendao-produced relations that have already been pruned, partitioned, and materialized by the Wendao Query Engine, spawning "Agent Workers" only for relevant data shards.
- **Relation-Aware Scheduling**: Agent steps consume typed relations and emit typed relations. Qianji schedules these steps without claiming ownership of DataFusion planner internals.

### 3.3 Relational Skepticism (Hallucination Detection)

The **Skeptic (Auditor)** role is upgraded from a prompt-based check to a **Relational Join** check.

- **Consistency Joins**: Qianji validates LLM-generated suggestions by issuing audit queries against Wendao relations and performing contradiction checks against Wendao-produced truth tables.
- **Example**: If an agent suggests a function signature change, Qianji requests the relevant truth relation from Wendao and verifies whether that signature already exists before admitting the suggestion into the workflow.

## 4. Proposed Workflow Stage Model

Qianji should introduce workflow stages, not a competing query operator model.

1. **`agent_step(relation, prompt_template)`**: Applies an agent-defined transformation to rows or row groups in an Arrow relation.
2. **`audit_step(relation, audit_goal)`**: Verifies agent outputs against Wendao truth relations or explicit invariants.
3. **`reduce_step(relation, goal)`**: Summarizes a columnar result set into a structured insight card.
4. **`graph_context_step(seed_relation, context_goal)`**: Requests graph-adjacent context from Wendao and enriches the workflow state with the returned relation.

These workflow stages must remain above the Wendao query layer. Qianji may compose Wendao queries, but it should not redefine retrieval or graph operators that already belong to Wendao.

## 5. Integration with Zhenfa Stream Processing

Qianji will leverage the **Unified Streaming Parser** from Zhenfa:

- **Streaming Telemetry**: As Wendao executes query plans and agents consume the returned relations, Qianji will emit `ZhenfaStreamingEvents` showing the "Data Funnel" in the UI (e.g., "Scanning 1M lines... 500 potential matches... LLM refining...").
- **Thought Separation**: Intercepting the `Thinking Process` of agents during the handoff, allowing the **Cognitive Supervisor** to halt workflows if the reasoning logic diverges from the data schema.

## 6. Implementation Phases

### Phase 1: Arrow Interface Handoff

- Implement `WorkflowContext` that holds `BTreeMap<String, Arc<RecordBatch>>`.
- Update Qianhuan Agent definitions to accept and return Arrow batches.

### Phase 2: Wendao Relation Integration

- Integrate Qianji workflow stages with Wendao query outputs and explain streams.
- Implement the `Relational Skeptic` by requesting audit relations from Wendao rather than embedding planner ownership in Qianji.

### Phase 3: Streaming Visualization

- Bind Wendao execution telemetry and workflow-stage telemetry to the Qianji UI.
- Show real-time "Data Flow" animations in the Workflow editor.

## 7. Success Metrics

- **Handoff Latency**: Inter-agent data transfer for 10MB of records < 1ms.
- **Audit Accuracy**: 95%+ of AST-related hallucinations caught by relation-backed audit checks.
- **User Trust**: Clear visual correlation between "Raw Data" and "AI Conclusion" in the Studio.

---

_Document Date: 2026-03-26_
_Status: RFC-QIANJI-001_
