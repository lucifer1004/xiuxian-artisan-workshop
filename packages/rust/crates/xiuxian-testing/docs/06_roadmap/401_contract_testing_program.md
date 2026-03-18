# Contract Testing Program V1 Roadmap

:PROPERTIES:
:ID: xiuxian-testing-contract-program-v1
:PARENT: [[../index]]
:TAGS: roadmap, rollout, contracts
:STATUS: ACTIVE
:END:

## Mission

Move `xiuxian-testing` from shared helper crate to a reusable engineering-governance kernel without collapsing into a vague lint bundle or an LLM-only review tool.

## Phase 0: Docs Kernel and Research Tracker

Delivered in this pass:

- docs kernel established
- research tracker seeded with post-2024 papers
- V1 architecture and rule-pack specification documented
- lightweight docs contract test added

## Phase 1: Contract Kernel Implementation

Target:

- add `contracts` module
- define `ContractFinding`, `ContractReport`, `RulePack`, and report serialization
- support `strict`, `advisory`, and `research` modes

Acceptance:

- one internal consumer can build a `ContractReport`
- findings serialize to JSON and render as markdown summaries

## Phase 2: `rest_docs` Rule Pack

Target:

- collect route metadata, OpenAPI, and endpoint docs
- emit deterministic findings for missing or inconsistent contract surface

Suggested first consumer:

- `xiuxian-wendao` gateway

Acceptance:

- deterministic rules can flag missing docs, missing examples, and schema or status drift
- outputs remain stable in CI

## Phase 3: `modularity` Rule Pack

Target:

- collect Rust module graph and visibility metadata
- enforce interface-only `mod.rs`, visibility discipline, and public API error docs

Suggested first consumers:

- `xiuxian-testing`
- one kernel crate such as `xiuxian-wendao`

Acceptance:

- findings can point to path-level architecture issues, not only local syntax warnings

## Phase 4: Wendao Knowledge Export

Target:

- export findings as ingestion-ready knowledge envelopes
- retain provenance, remediation, and examples

Acceptance:

- Wendao can index findings in a stable schema
- retrieval surfaces can answer "what engineering contracts are currently drifting?"

## Phase 5: Runtime and Review Feedback

Target:

- integrate runtime traces or invariants
- experiment with review-guided test targeting

This phase depends on V1 proving the base schema and deterministic rule flow.

## Immediate Next Steps

1. Implement the `contracts` data model in `xiuxian-testing`.
2. Land a minimal markdown and JSON reporter.
3. Prototype `rest_docs` against `xiuxian-wendao` gateway endpoints.
4. Prototype `modularity` against `xiuxian-testing` itself.

## Exit Criteria for the V1 Design Stage

The design stage is complete when:

- the kernel schema is agreed and coded,
- the first two rule packs exist in advisory mode,
- at least one crate receives contract reports in CI,
- Wendao export shape is fixed enough for later ingestion work.
