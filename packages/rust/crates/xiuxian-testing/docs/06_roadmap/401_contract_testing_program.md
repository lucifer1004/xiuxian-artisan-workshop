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

Delivered seed in this pass:

- `ModularityRulePack` implemented in `xiuxian-testing` with deterministic checks for:
  - `MOD-R001`: non-interface logic inside `mod.rs`
  - `MOD-R002`: broad `pub` visibility in internal module files
  - `MOD-R003`: public `Result` APIs missing `# Errors` docs

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

## Current Downstream Consumer Evidence

`xiuxian-llm` is now a concrete downstream example of the kind of executable contract surface this
kernel should eventually formalize.

Current downstream DeepSeek OCR Metal short-form ladder under the shared `12 GB` guard:

- `0`
- `Telegram`
- `Telegram OCR`
- `sidecar health check`
- `Managed sidecar health check`

These gates are profile-backed, evidence-addressable, and already encode a narrow semantic
contract through `expected_substring`. The fourth gate combines a passing manual probe with an
observed successful profile-backed rerun, and the fifth has a direct file-backed profile pass.
The memory-line branch remains exploratory in the latest reruns and is not currently retained
under the same `12 GB` guard. These gates are not yet produced by `xiuxian-testing`,
but they are a live consumer signal for why the future contract kernel needs stable finding
schemas, profile-aware evidence capture, and docs-backed traceability.

Current non-retained follow-ups are also informative for the future contract kernel: a direct
`Hello from Telegram OCR.` line probe stayed within the same `12 GB` guard but fell into a long
low-RSS tail instead of converging quickly. A later title-line rerun for
`Omni OCR smoke test` passed `capfox` and stayed within the same `12 GB` guard, but also fell into
a long low-RSS tail (`~1.67 GB` for more than `100s`) without converging to a retained output.
Another short title-line variant (`.run/tmp/downstream_deepseek_metal_omni_line_probe_12g_v3.log`)
now reproduces a direct guard breach. For the memory-line branch, `max_new_tokens=3` still
reproduces the same guard breach, and later `max_new_tokens=2` reruns in the same snapshot also
regress to guard breaches.
Together these runs show that downstream evidence must distinguish semantic long-tail rejection
from ambient-capacity denial.

The runner surface is also now GPU-backend aware rather than Metal-only. `xiuxian-llm` can expose
the same ignored real-GPU harness as `test_real_metal_inference` or `test_real_cuda_inference`,
and the Python runner can request `--cuda` while reusing the same contract fields. That broadens
the future contract-kernel shape, but the retained downstream evidence in this workspace snapshot
is still Metal-backed because no local CUDA proof has been captured yet.

## Immediate Next Steps

Delivered in this pass:

1. `qianji` scenario-audit (`formal_audit`) is now a named contract-feedback gate.
2. `xiuxian-testing` contract packs are enforced through CI-visible gates.
3. `xiuxian-wendao` now exposes a stable bundled gateway `OpenAPI` artifact for clean `rest_docs` validation.
4. One persisted `qianji -> wendao` downstream proof now exists through
   `wendao_persisted_rest_docs_contract_feedback`, and the strengthened
   `xiuxian_wendao_contract_feedback_consumer.sh` gate covers both adapter
   mapping and sink persistence.

Remaining next steps:

1. Stabilize Wendao export ingestion around `ContractKnowledgeBatch` and define one retrieval query for active drift tracking.
2. Expand contract-pack CI adoption to one more kernel crate after `xiuxian-wendao`.

## Snapshot Governance Notes

- `xiuxian-testing::scenario::ScenarioFramework` now fails closed on duplicate scenario ids before
  snapshot assertion. This prevents accidental Insta snapshot collisions when two fixtures reuse the
  same `id`.
- Current verification evidence:
  - `direnv exec . env CARGO_TARGET_DIR=.cache/cargo-target/xiuxian-testing-nextest cargo nextest run -p xiuxian-testing --lib --tests --no-fail-fast`
  - `direnv exec . bash scripts/rust/xiuxian_qianji_scenario_audit_contracts.sh`
  - `direnv exec . bash scripts/rust/xiuxian_testing_contract_gates.sh`
  - `direnv exec . env CARGO_TARGET_DIR=.cache/cargo-target/wendao-live-openapi-gate bash scripts/rust/xiuxian_wendao_live_openapi_contract_feedback.sh`
  - `direnv exec . env CARGO_TARGET_DIR=.cache/cargo-target/wendao-contract-feedback-consumer bash scripts/rust/xiuxian_wendao_contract_feedback_consumer.sh`
  - `direnv exec . env CARGO_TARGET_DIR=.cache/cargo-target/wendao-contract-feedback-consumer cargo test -p xiuxian-qianji --test wendao_persisted_rest_docs_contract_feedback`
  - `direnv exec . env CARGO_TARGET_DIR=.cache/cargo-target/xiuxian-testing-nextest cargo clippy -p xiuxian-testing --all-targets -- -D warnings`
  - `direnv exec . bash scripts/ci_scripts_smoke.sh`
  - `direnv exec . env CARGO_TARGET_DIR=.cache/cargo-target/wendao-scenarios-nextest cargo nextest run -p xiuxian-wendao --test scenarios_test --no-fail-fast`

## Exit Criteria for the V1 Design Stage

The design stage is complete when:

- the kernel schema is agreed and coded,
- the first two rule packs exist in advisory mode,
- at least one crate receives contract reports in CI,
- Wendao export shape is fixed enough for later ingestion work.
