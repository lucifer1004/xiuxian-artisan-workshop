---
type: knowledge
title: "Qianji Prototype Design: Autonomous Audit & Remediation Pipeline"
category: "architecture"
tags:
  - qianji
  - multi-agent
  - automation
  - audit
  - ralph-workflow
  - wendao-support
saliency_base: 8.5
decay_rate: 0.01
metadata:
  title: "Qianji Prototype Design: Autonomous Audit & Remediation Pipeline"
---

# Qianji Prototype Design: Autonomous Audit & Remediation Pipeline

## 1. Vision: The Unattended Sovereign Auditor

Inspired by the `Ralph-Workflow` philosophy of long-running, unattended development loops, this design elevates **Qianji** from a simple prompt orchestrator to an autonomous engineering lifecycle manager.

The goal is to enable a **"Submit Prompt → Autonomous Audit → Auto-Remediation → Verified Commit"** pipeline that can run for hours without human intervention, governed by the **Auditor's Codex**.

## 2. Core Orchestration Loop (The Trinity Cycle)

The Qianji pipeline implements a specialized version of the Plan-Develop-Verify cycle, optimized for the **CyberXiuXian** environment:

1.  **[PHASE-PLAN]**: **Gemini (The Architect)**
    - **Input**: `PROMPT.md` + Repository Context (via `omni-agent`).
    - **Constraint**: Must output a structured XML plan validated against `qianji_plan.xsd`.
    - **Output**: `PLAN.xml` (containing file targets, risk mitigations, and verification strategies).
2.  **[PHASE-EXECUTE]**: **Claude (The Senior Engineer)**
    - **Input**: `PLAN.xml` + Specific file access.
    - **Operation**: Parallel tool calls to modify files as per the plan.
    - **Constraint**: Strict adherence to the **Include Pattern** for new modules.
3.  **[PHASE-VERIFY]**: **Gemini/Claude (The Auditor)**
    - **Input**: Changed files + `PLAN.xml` verification strategy.
    - **Operation**: Run `TIER-3: GATE` (Clippy, Nextest, Security Scans).
    - **Loop**: If verification fails, feed logs back to Phase 2 (Remediation) with a max-retry limit of 5.

## 3. Technical Implementation Standards

### 3.1. Structured Output (XSD Enforcement)

To eliminate hallucination spirals, all agent communications in Qianji MUST be validated via XSD.

- **`qianji_plan.xsd`**: Enforces quantified scopes and mandatory risk assessment.
- **`qianji_audit_report.xsd`**: Enforces specific vulnerability categories and severity levels.

### 3.2. Engineering Excellence (Include Pattern)

The Rust core implementation of the Qianji Orchestrator MUST follow the **Include Pattern** to maintain hyper-modularity:

- `qianji-executor/src/pipeline.rs`: The orchestrator.
- `qianji-executor/src/pipeline/planner.rs`: Phase 1 logic.
- `qianji-executor/src/pipeline/developer.rs`: Phase 2 logic.
- `qianji-executor/src/pipeline/auditor.rs`: Phase 3 logic.

### 3.3. Wendao Knowledge Graph Integration

Every autonomous run generates an **"Execution Artifact"** which is ingested into Wendao:

- **Relationship Mapping**: Connects the `PROMPT.md` (Intent) to the `PLAN.xml` (Strategy) and the final `COMMIT` (Outcome).
- **Audit History**: Provides a searchable history of detected vulnerabilities and their corresponding automated fixes.

## 4. Wendao-Compatible Metadata Specification

For full indexing support in **Wendao**, this document and all subsequent Qianji reports must include the following metadata headers:

- `type`: `knowledge` (for persistent designs) or `artifact` (for run results).
- `saliency_base`: Higher values (8+) for core architectural designs.
- `decay_rate`: Low (0.01) for foundational specs; higher for transient execution logs.

## 5. Next Steps for Implementation

1.  **Draft `qianji_plan.xsd`**: Define the mandatory structure for the autonomous planning phase.
2.  **Scaffold `xiuxian-qianji`**: Apply the **Include Pattern** to create the multi-agent pipeline skeleton in Rust.
3.  **Implement NDJSON Parser**: Port the `Ralph-Workflow` streaming parser logic to handle real-time feedback from Claude/Gemini CLIs.

---

_Status: DRAFT (V1.0)_
_Author: CyberXiuXian Artisan Studio_
_Reference: Ralph-Workflow Research Artifact, Auditor's Codex_
