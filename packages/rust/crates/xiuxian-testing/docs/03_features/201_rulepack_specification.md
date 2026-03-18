# Rule-Pack Specification V1

:PROPERTIES:
:ID: xiuxian-testing-rulepack-spec-v1
:PARENT: [[../index]]
:TAGS: specification, rulepack, findings
:STATUS: ACTIVE
:END:

## Purpose

Define a stable specification for how `xiuxian-testing` rule packs declare rules, collect evidence, emit findings, and map severities into CI behavior.

## Rule-Pack Contract

Each rule pack should expose:

- a stable pack identifier
- a version
- a declared evidence scope
- a deterministic evaluation entrypoint
- an optional advisory heuristic entrypoint

Suggested shape:

```rust
pub trait RulePack {
    fn descriptor(&self) -> RulePackDescriptor;
    fn collect(&self, ctx: &CollectionContext) -> anyhow::Result<CollectedArtifacts>;
    fn evaluate(&self, artifacts: &CollectedArtifacts) -> anyhow::Result<Vec<ContractFinding>>;
}

pub struct RulePackDescriptor {
    pub id: &'static str,
    pub version: &'static str,
    pub domains: &'static [&'static str],
    pub default_mode: FindingMode,
}
```

Rule packs may optionally request an advisory audit pass after deterministic findings are produced. That execution should flow through the integration described in [[202_multi_role_audit_integration]].

## Finding Model

Every finding should be complete enough for:

- terminal output
- CI annotations
- markdown reports
- Wendao ingestion

Suggested shape:

```rust
pub struct ContractFinding {
    pub rule_id: String,
    pub pack_id: String,
    pub severity: FindingSeverity,
    pub mode: FindingMode,
    pub confidence: FindingConfidence,
    pub advisory_role_ids: Vec<String>,
    pub trace_ids: Vec<String>,
    pub title: String,
    pub summary: String,
    pub why_it_matters: String,
    pub remediation: String,
    pub evidence: Vec<FindingEvidence>,
    pub examples: FindingExamples,
    pub labels: BTreeMap<String, String>,
}
```

Recommended enums:

```rust
pub enum FindingSeverity {
    Info,
    Warning,
    Error,
    Critical,
}

pub enum FindingMode {
    Deterministic,
    Advisory,
    Research,
}

pub enum FindingConfidence {
    High,
    Medium,
    Low,
}
```

Suggested companion request for packs that opt into advisory execution:

```rust
pub struct AdvisoryAuditPolicy {
    pub enabled: bool,
    pub requested_roles: Vec<String>,
    pub min_severity: FindingSeverity,
}
```

## Evidence Model

Evidence should stay explicit and path-addressable:

```rust
pub struct FindingEvidence {
    pub kind: EvidenceKind,
    pub path: Option<PathBuf>,
    pub locator: Option<String>,
    pub message: String,
}
```

Suggested evidence kinds:

- `SourceSpan`
- `OpenApiNode`
- `DocSection`
- `RuntimeTrace`
- `ScenarioSnapshot`
- `DerivedInvariant`

## V1 Rule Packs

### `rest_docs`

Scope:

- OpenAPI, route declarations, handlers, gateway docs, examples

Initial rule set:

- `REST-R001`: every externally reachable endpoint has a stable purpose description
- `REST-R002`: request schema and handler parameter surface are consistent
- `REST-R003`: documented success and error status codes match implementation behavior contracts
- `REST-R004`: error payloads use one documented envelope shape per service surface
- `REST-R005`: pagination and filtering semantics are documented when collection endpoints expose them
- `REST-R006`: mutation endpoints declare idempotency behavior or explicit non-idempotency
- `REST-R007`: at least one realistic example exists for non-trivial request bodies

### `modularity`

Scope:

- Rust module graph, visibility, crate boundaries, internal adapters

Initial rule set:

- `MOD-R001`: `mod.rs` remains interface-only
- `MOD-R002`: internal collaboration defaults to `pub(crate)` unless a public surface is required
- `MOD-R003`: forbidden cross-layer imports are rejected
- `MOD-R004`: public `Result` APIs include explicit `# Errors` documentation
- `MOD-R005`: gateway and adapter layers do not own domain logic that belongs in kernel modules

### `knowledge_feedback`

Scope:

- exported findings, remediation examples, Wendao ingestion payloads

Initial rule set:

- `KNOW-R001`: every error-level finding contains actionable remediation
- `KNOW-R002`: every exported finding includes source provenance
- `KNOW-R003`: advisory findings include confidence metadata
- `KNOW-R004`: pack outputs are serializable into Wendao-facing knowledge envelopes

### `multi_role_audit`

Scope:

- role execution metadata, trace attribution, advisory audit provenance

Initial rule set:

- `AUDIT-R001`: advisory findings retain the originating `role_id`
- `AUDIT-R002`: advisory findings that influence remediation include a linked `trace_id`
- `AUDIT-R003`: cognitive-drift or early-halt conditions from audit runs are surfaced in the final report
- `AUDIT-R004`: role-based critique is never promoted to deterministic pass/fail without an explicit deterministic rule backing it

## Severity Policy

V1 should treat severity as policy-configurable:

- `Info`: record only
- `Warning`: advisory by default, optionally gate in strict mode
- `Error`: fail in strict mode
- `Critical`: fail in all non-research modes

This allows frontier rule packs to start in advisory mode and harden over time.

## Deterministic vs Advisory Split

A single rule pack may include both classes of rules, but they must remain distinguishable:

- deterministic rules decide on explicit structural evidence
- advisory rules interpret underspecified or natural-language-heavy evidence

Example:

- deterministic: endpoint exists in code but is missing from the OpenAPI document
- advisory: endpoint description is too vague for safe external use
- advisory multi-role: the `rest_contract_auditor` and `runtime_trace_reviewer` both flag a suspicious contract, but the final severity remains policy-driven until deterministic evidence confirms the drift

The split must be preserved in the report schema so teams can gate only on the signals they trust.

## Reporting Contract

V1 reports should support:

- summary counts by severity and pack
- grouped findings
- stable machine-readable identifiers
- reproducible file references
- export to markdown and JSON

Recommended top-level JSON shape:

```json
{
  "suite_id": "xiuxian-testing-contracts",
  "generated_at": "2026-03-17T00:00:00Z",
  "findings": [],
  "stats": {
    "critical": 0,
    "error": 0,
    "warning": 0,
    "info": 0
  }
}
```

## Minimum V1 Pilot

Pilot the architecture in two places:

1. `xiuxian-wendao` gateway surface for `rest_docs`
2. `xiuxian-testing` and one Rust kernel crate for `modularity`
3. `qianji + qianhuan + zhenfa + wendao` as the advisory execution lane for selected `rest_docs` findings

This keeps the first implementation focused while still proving the full contract path from code and docs to findings and knowledge export.
