# Multi-Role Audit Integration V1

:PROPERTIES:
:ID: xiuxian-testing-multi-role-audit-v1
:PARENT: [[../index]]
:TAGS: qianji, qianhuan, zhenfa, wendao, advisory
:STATUS: ACTIVE
:END:

## Purpose

Define how the existing multi-role audit capabilities in `xiuxian-qianji` and `xiuxian-qianhuan` plug into the `xiuxian-testing` V1 contract kernel without duplicating orchestration logic.

The design goal is explicit:

- `xiuxian-testing` owns contract schemas and deterministic rule evaluation
- `qianhuan` owns role and persona manifestation
- `qianji` owns audit execution and retry flow
- `zhenfa` owns streaming evidence normalization
- `wendao` owns trace persistence and retrieval

## Existing Runtime Capabilities

### `xiuxian-qianhuan`: Persona and role manifestation

Relevant surfaces:

- `ThousandFacesOrchestrator`
- `PersonaRegistry`
- layered prompt injection and XML output discipline

This is the right place to define role specialization such as:

- REST contract auditor
- modularity auditor
- documentation consistency auditor
- runtime trace reviewer

### `xiuxian-qianji`: Formal audit execution loop

Relevant surfaces:

- `formal_audit`
- `LlmAugmentedAuditMechanism`
- threshold-based retry and abort policies

This is the right place to run role-specific audits, aggregate outputs, and decide whether a critique should trigger retries or escalation.

### `xiuxian-zhenfa`: Unified streaming evidence

Relevant surfaces:

- `ZhenfaStreamingEvent`
- `ZhenfaPipeline`
- `CognitiveSupervisor`

This is the right place to normalize provider output into a shared event stream and to capture cognitive drift or coherence metrics during advisory audit runs.

### `xiuxian-wendao`: Persistent audit memory

Relevant surfaces:

- `CognitiveTraceRecord`
- `LinkGraphSemanticDocumentKind::CognitiveTrace`
- `to_semantic_document()`

This is the right place to persist the audit reasoning chain and later connect it to contract findings and remediation history.

## V1 Integration Model

The V1 contract system should treat multi-role audits as an advisory execution layer, not as the source of truth.

```text
Deterministic Contract Rules
        │
        ├── PASS / FAIL / WARN
        │
        ▼
Advisory Multi-Role Audit Request
        │
        ▼
Qianhuan role manifestation
        │
        ▼
Qianji formal audit execution
        │
        ▼
Zhenfa unified streaming + cognitive supervision
        │
        ▼
Wendao CognitiveTrace persistence
        │
        ▼
Advisory findings merged into ContractReport
```

This ordering matters. Deterministic evidence should establish the baseline before any role-based interpretation runs.

## Proposed Interfaces

`xiuxian-testing` should define the contract-facing advisory interface, but it should not implement persona orchestration itself.

```rust
pub trait AdvisoryAuditExecutor {
    async fn run(
        &self,
        request: AdvisoryAuditRequest,
    ) -> anyhow::Result<Vec<RoleAuditFinding>>;
}

pub struct AdvisoryAuditRequest {
    pub suite_id: String,
    pub pack_id: String,
    pub findings: Vec<ContractFinding>,
    pub artifacts: CollectedArtifacts,
    pub requested_roles: Vec<String>,
}

pub struct RoleAuditFinding {
    pub role_id: String,
    pub rule_id: Option<String>,
    pub severity: FindingSeverity,
    pub confidence: FindingConfidence,
    pub summary: String,
    pub why_it_matters: String,
    pub remediation: String,
    pub evidence: Vec<FindingEvidence>,
    pub trace_id: Option<String>,
}
```

The key boundary is:

- `ContractFinding` is the stable kernel schema
- `RoleAuditFinding` is the role-attributed advisory supplement

## Role Mapping for V1

Recommended first role set:

- `rest_contract_auditor`
  Focus: endpoint purpose, request or response consistency, examples, status codes
- `modularity_auditor`
  Focus: layer boundaries, visibility, ownership drift, adapter leakage
- `doc_consistency_auditor`
  Focus: public API docs, contract language, mismatches between docs and behavior
- `runtime_trace_reviewer`
  Focus: trace quality, cognitive drift, suspicious tool usage or fragile execution paths

These roles should be configured through `qianhuan`, not hard-coded into `xiuxian-testing`.

## Audit Trace Lifecycle

1. `xiuxian-testing` runs deterministic packs and produces baseline findings.
2. Selected findings are bundled into an `AdvisoryAuditRequest`.
3. `qianhuan` manifests the requested role prompts and output constraints.
4. `qianji` runs the formal audit flow with threshold and retry control.
5. `zhenfa` captures streaming output and cognitive metrics during the run.
6. `ThoughtAggregator` builds a `CognitiveTraceRecord`.
7. `ArtifactObserver` ingests the trace into `Wendao`.
8. The resulting `trace_id` is attached back onto `RoleAuditFinding`.
9. `xiuxian-testing` merges advisory outputs into the final `ContractReport`.

This gives every advisory claim an auditable reasoning chain.

## Why This Matters

Without this integration, role-based audit is just ephemeral text generation.

With this integration:

- critiques become attributable to a concrete role
- critiques carry runtime trace provenance
- critiques can be revisited through Wendao search
- remediation history can be linked to the exact advisory reasoning that proposed it

That is the difference between "LLM review" and a reusable knowledge-producing audit system.

## Guardrails

V1 should keep these boundaries strict:

- multi-role audit never replaces deterministic contract checks
- low-confidence role findings stay advisory by default
- `Wendao` stores traces and findings, but does not decide pass or fail on its own
- retry loops remain owned by `qianji formal_audit`, not by the test kernel

## Immediate Follow-Up

The first implementation target should be:

1. `rest_docs` deterministic findings on `xiuxian-wendao`
2. advisory `rest_contract_auditor` execution through `qianji + qianhuan`
3. trace persistence into `Wendao`
4. merged reporting back into one `ContractReport`

This proves the full path from contract detection to multi-role critique to persistent audit memory.
