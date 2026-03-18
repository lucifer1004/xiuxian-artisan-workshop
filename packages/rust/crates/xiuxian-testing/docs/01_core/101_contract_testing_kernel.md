# Contract Testing Kernel V1

:PROPERTIES:
:ID: xiuxian-testing-contract-kernel-v1
:PARENT: [[../index]]
:TAGS: architecture, contracts, testing
:STATUS: ACTIVE
:END:

## Goal

Evolve `xiuxian-testing` from a shared helper crate into a contract-testing kernel that can:

1. audit engineering structure and documentation contracts,
2. convert REST and documentation expectations into machine-checkable findings,
3. produce reusable evidence for human review and Wendao ingestion, and
4. separate deterministic checks from LLM-assisted heuristics.

## Design Principles

- Deterministic checks first. LLMs may suggest or classify, but they do not own pass/fail on their own.
- Contracts are explicit artifacts. Tests should consume normalized contract data, not free-form prose alone.
- Findings must be reusable. Every violation should carry machine-readable evidence, remediation guidance, and provenance.
- Rule packs stay modular. REST, modularity, and knowledge-export concerns should evolve independently.
- Advisory and strict modes must coexist. Teams need a path from warnings to hard gates.
- Runtime and role evidence are first-class. If advisory audits run through `Qianji` and `Zhenfa`, their traces should remain attachable to final findings and exportable to `Wendao`.

## Existing Base Inside `xiuxian-testing`

The current crate already has three usable foundations:

- `scenario`: reusable scenario and snapshot execution
- `policy`: crate-level test structure policy
- `external_test` and `validation`: convention validation for externalized tests and filesystem structure

V1 should build on these surfaces instead of replacing them.

## Proposed V1 Layers

### 1. Structure Policy Layer

This is the current deterministic base:

- crate test layout validation
- external test mount policy
- snapshot policy and redaction support

This layer remains purely deterministic and should keep running in CI as the low-risk baseline.

### 2. Contract Kernel Layer

Add a shared model for contract execution:

```rust
pub struct ContractSuite {
    pub id: String,
    pub version: String,
    pub rule_packs: Vec<Box<dyn RulePack>>,
}

pub trait RulePack {
    fn id(&self) -> &'static str;
    fn collect(&self, ctx: &CollectionContext) -> anyhow::Result<CollectedArtifacts>;
    fn evaluate(&self, input: &CollectedArtifacts) -> anyhow::Result<Vec<ContractFinding>>;
}

pub struct ContractFinding {
    pub rule_id: String,
    pub severity: FindingSeverity,
    pub mode: FindingMode,
    pub title: String,
    pub summary: String,
    pub why_it_matters: String,
    pub remediation: String,
    pub evidence: Vec<FindingEvidence>,
    pub source_paths: Vec<PathBuf>,
}
```

This layer is the stable spine for all future rule packs.

### 3. Artifact Collection Layer

Normalize inputs before evaluation:

- source code structure
- Rust module graph
- OpenAPI documents
- inline and external engineering documentation
- runtime traces or logs when available
- Wendao-exportable knowledge envelopes

The collector layer is where code, docs, and runtime evidence become comparable artifacts.

### 4. Rule-Pack Execution Layer

Each rule pack evaluates a bounded concern:

- `rest_docs`
- `modularity`
- `knowledge_feedback`
- later: `runtime_invariants`, `review_guidance`, `scenario_quality`

Every rule pack returns findings through the same kernel schema.

### 4.5 Advisory Audit Execution Layer

This layer reuses the existing runtime stack rather than rebuilding it inside `xiuxian-testing`:

- `Qianhuan` manifests the requested auditor roles
- `Qianji formal_audit` executes the advisory critique loop
- `ZhenfaPipeline` normalizes streaming output and cognitive metrics
- `Wendao` stores resulting `CognitiveTrace` artifacts

The contract kernel should treat this layer as an optional role-attributed supplement to deterministic findings, not as a replacement.

### 5. Reporting and Export Layer

V1 should emit two stable outputs:

- human-readable markdown or terminal summaries
- machine-readable JSON for CI, dashboards, and Wendao indexing

Suggested report surface:

```rust
pub struct ContractReport {
    pub suite_id: String,
    pub generated_at: String,
    pub findings: Vec<ContractFinding>,
    pub stats: ContractStats,
}
```

### 6. Wendao Feedback Layer

This is where the testing system becomes a knowledge system.

Every exported finding should include:

- `rule_id`
- `domain`
- `severity`
- `decision` (`pass`, `warn`, `fail`)
- `evidence_excerpt`
- `why_it_matters`
- `remediation`
- `good_example`
- `bad_example`
- `source_path`

This keeps test output usable by both humans and retrieval systems.

## Execution Modes

V1 should support three modes:

- `strict`: fail the run on configured severities
- `advisory`: report findings without failing
- `research`: collect rich evidence, retain low-confidence findings, and support paper-driven exploration

This lets the same architecture support production gating and frontier research.

Within `advisory` mode, multi-role audit is the main consumer of the runtime stack described in [[../03_features/202_multi_role_audit_integration]].

## Where REST Best Practices Fit

REST engineering quality should not be treated as one monolithic lint. V1 should model it as a contract family:

- route purpose and naming
- request and response schema completeness
- status-code coverage
- error-envelope consistency
- pagination and filtering semantics
- idempotency and mutation semantics
- examples and documentation depth
- code, docs, and OpenAPI consistency

That family becomes one rule pack, not the whole system.

## Where Modularity Fits

Modularity should be audited from source structure and visibility, not only style lints.

Examples:

- `mod.rs` interface-only discipline
- `pub(crate)` default for internal boundaries
- forbidden cross-layer imports
- adapter versus kernel boundary checks
- doc coverage on public `Result` APIs

This is how the system grows from code-style checking into architecture governance.

## Non-Goals for V1

- full automatic OpenAPI inference
- full runtime invariant mining
- autonomous rule remediation
- LLM-only pass/fail decisions

These belong to later phases after the contract schema and rule-pack interfaces stabilize.

## V1 Acceptance Signal

V1 is successful when:

1. at least one crate can run deterministic contract checks for REST docs and modularity,
2. findings share one schema across rule packs,
3. advisory findings can be attributed to role-based audit runs from `Qianji` and `Qianhuan`,
4. findings and traces are exportable to Wendao as knowledge records, and
5. advisory and strict execution modes are both supported at the design level.
