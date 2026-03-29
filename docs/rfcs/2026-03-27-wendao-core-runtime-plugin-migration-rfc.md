---
type: knowledge
title: "RFC: Wendao Core Runtime and Arrow Plugin Migration"
category: "rfc"
status: "draft"
authors:
  - codex
created: 2026-03-27
tags:
  - rfc
  - wendao
  - plugins
  - arrow
  - datafusion
  - runtime
  - migration
metadata:
  title: "RFC: Wendao Core Runtime and Arrow Plugin Migration"
---

# RFC: Wendao Core Runtime and Arrow Plugin Migration

## 1. Summary

This RFC defines a complete migration of Wendao from a crate that currently mixes core contracts, runtime orchestration, gateway assembly, and language-specific plugin details into a layered architecture with four stable roles:

1. `xiuxian-wendao-core`
2. `xiuxian-wendao-runtime`
3. independently published plugin packages such as `xiuxian-wendao-julia`
4. optional compatibility facade crates and binaries during migration

The architectural rule is:

**Arrow is the plugin data plane, DataFusion is the execution kernel, and language plugins are protocol providers rather than in-process Rust traits compiled into the host.**

This RFC also defines the migration principles needed to move incrementally without breaking current Julia-based functionality or future Python/JavaScript plugin adoption.

## 2. Motivation

Wendao already has a clear center of gravity:

1. `Arrow IPC` is the current high-performance exchange boundary.
2. `Arrow Flight` is the preferred future transport.
3. `DataFusion` is the correct host execution kernel.
4. Julia-based analyzers and rerankers already prove that language-native Arrow services are a viable integration model.

The current issue is not lack of capability. The issue is boundary drift.

Today, `xiuxian-wendao` simultaneously owns:

1. repository-intelligence contracts
2. runtime orchestration
3. gateway- and Studio-facing types
4. language-specific runtime config
5. language-specific deployment artifact surfaces
6. in-tree compilation of sibling plugin sources

This creates three structural risks:

1. every new language or analyzer type will enlarge the core crate
2. the host API will become polluted with language-specific types
3. independently published plugins will remain a goal rather than a stable architecture

## 3. Problem Statement

The current architecture lacks a stable separation between:

1. core protocol contracts
2. runtime behavior
3. plugin packages
4. deployment and artifact surfaces

If not corrected, Wendao will accumulate repeated special cases:

1. `Julia*` runtime structs in core
2. `Python*` runtime structs in core
3. `Js*` runtime structs in core
4. repeated transport and artifact logic across language paths

This would undermine the two most important long-term goals:

1. independent plugin publication
2. Arrow/DataFusion-first execution consistency

## 4. Goals

This RFC has the following goals:

1. Define the target crate topology for Wendao.
2. Define the stable plugin protocol boundary.
3. Define what belongs in `core` versus `runtime`.
4. Preserve Julia performance and current Arrow IPC viability during migration.
5. Make future Python and JavaScript plugin adoption additive instead of invasive.
6. Define a phased migration path with compatibility windows.

## 5. Non-Goals

This RFC does not attempt to:

1. finish the code refactor in one landing
2. force immediate Flight support for all plugin runtimes
3. eliminate all compatibility re-exports on day one
4. define every query capability in final detail
5. replace current working Julia integrations before replacements exist

## 6. Architectural Principles

### 6.1 Capability-First Host Modeling

The host core should model plugin functionality by capability, not by language.

Examples:

1. `rerank`
2. `analyze_repository`
3. `feature_extract`
4. `artifact_export`
5. `table_provider`

### 6.2 Arrow-First Data Plane

All high-value plugin capabilities must use Arrow-native request and response contracts.

This means:

1. `RecordBatch` and batch streams are the default data boundary
2. JSON and TOML remain control-plane formats
3. schema governance is first-class

### 6.3 DataFusion-Oriented Host Execution

The host runtime should integrate plugin capabilities as DataFusion-oriented execution units rather than arbitrary RPC helpers.

At minimum, the runtime should preserve:

1. schema validation
2. projection-aware request shaping
3. bounded materialization
4. pushdown-aware capability negotiation

### 6.4 Plugin Packages Are Independently Published Units

Language-specific support must live in plugin packages that can be published, versioned, installed, and upgraded separately from the host core.

### 6.5 Runtime Logic Is Not Core API

Transport negotiation, process lifecycle, health checks, config resolution, and fallback handling are runtime responsibilities. They must not define the stable host API.

### 6.6 Feature-Boundary-First Modularization

Wendao migration is feature-boundary-first, not crate-split-first.

That means:

1. new modules must be organized by functional responsibility, not by arbitrary file growth
2. medium or complex features must prefer a directory namespace over a flat file
3. namespace names must reflect intent, not accidental implementation history
4. `mod.rs` must remain interface-only
5. physical crate splitting without logical modularization is explicitly disallowed

The migration must not replace one monolith with several smaller but still mixed-responsibility monoliths.

### 6.7 Responsibility-Oriented Naming

File and namespace names must communicate ownership clearly.

Good names describe one bounded responsibility, for example:

1. `manifest.rs`
2. `negotiation.rs`
3. `health.rs`
4. `launch.rs`
5. `records.rs`

Bad names hide mixed ownership or become catch-all sinks, for example:

1. `utils.rs`
2. `misc.rs`
3. `common.rs`
4. broad `helpers.rs` files without one cohesive helper domain

Shared helper files are acceptable only when the helper surface is both cohesive and tightly bounded by one responsibility.

DTO warehouses and mixed transport-orchestration files are migration smells and must be treated as refactor targets rather than acceptable end states.

## 7. Target Topology

## 7.1 Target Crates

### `xiuxian-wendao-core`

Owns stable contracts only.

It should include:

1. capability identifiers and versioning
2. plugin manifest types
3. plugin artifact types
4. transport descriptors
5. schema descriptors
6. shared records and query contracts
7. DataFusion-facing adapter traits and host-side abstractions

It should not include:

1. process launch code
2. runtime config discovery
3. plugin installation logic
4. gateway-specific response wrappers
5. language-specific deployment structs

### `xiuxian-wendao-runtime`

Owns host behavior.

It should include:

1. plugin discovery and installation metadata loading
2. manifest resolution and compatibility checks
3. transport negotiation
4. plugin process lifecycle
5. fallback routing
6. plugin health and readiness
7. telemetry and diagnostics
8. runtime config resolution
9. gateway and CLI assembly helpers where applicable

### Plugin Packages

Examples:

1. `xiuxian-wendao-julia`
2. `xiuxian-wendao-modelica`
3. future `xiuxian-wendao-python`
4. future `xiuxian-wendao-js`

Each plugin package should own:

1. plugin manifest
2. capability declarations
3. capability-specific Arrow schemas
4. launcher/runtime integration details
5. plugin-owned artifacts such as deployment manifests

### Compatibility Layer

During migration, `xiuxian-wendao` may temporarily remain as:

1. a facade crate
2. a compatibility re-export layer
3. a binary entrypoint that delegates to `runtime`

This is acceptable as a migration bridge, not as the final architectural truth.

## 7.2 Target Directory Boundaries

The target logical structure is:

```text
xiuxian-wendao-core
  - capabilities/
  - contracts/
  - schemas/
  - artifacts/
  - transport/
  - records/

xiuxian-wendao-runtime
  - discovery/
  - install/
  - lifecycle/
  - negotiation/
  - launch/
  - health/
  - telemetry/
  - gateway/
  - cli/

xiuxian-wendao-julia
  - plugin.toml
  - capabilities/rerank/
  - capabilities/analyze_repository/
  - artifacts/deployment/
  - launch/
```

## 8. Program Rollout Plan

This migration must now be executed as one coordinated program rather than as
disconnected local refactors.

The controlling rule is:

**All future implementation work must attach to one macro phase, one gate, and
one program-level success condition.**

### 8.1 Macro Phases

#### Phase M1: Contract and Compatibility Stabilization

Purpose:

1. finish in-place generalization of host-side contracts
2. stop new language-specific host vocabulary from appearing
3. consolidate compatibility seams into explicit namespaces

Completion conditions:

1. all new host-facing work uses capability-, artifact-, provider-, or
   transport-oriented vocabulary
2. crate-root compatibility exports are routed through explicit compatibility
   namespaces
3. no new Julia-specific host types are introduced outside compatibility seams

#### Phase M2: Core Boundary Extraction

Purpose:

1. move stable contracts into `xiuxian-wendao-core`
2. prove that `core` compiles without runtime lifecycle dependencies
3. establish semver governance for stable plugin contracts

Completion conditions:

1. `xiuxian-wendao-core` exists and builds
2. `core` owns capability, artifact, manifest, schema, and transport-descriptor
   contracts
3. compatibility re-exports exist where needed, but the ownership move is
   physical, not only conceptual

#### Phase M3: Runtime Boundary Extraction

Purpose:

1. move orchestration and lifecycle concerns into `xiuxian-wendao-runtime`
2. centralize transport negotiation and fallback handling
3. remove host-behavior ownership from the future `core` crate

Completion conditions:

1. `xiuxian-wendao-runtime` exists and builds
2. runtime owns launch, health, readiness, negotiation, fallback, and routing
3. binaries and host assembly paths delegate through runtime-owned seams

#### Phase M4: Julia Ownership Externalization

Purpose:

1. move Julia-specific ownership into `xiuxian-wendao-julia`
2. remove in-tree source inclusion and host-owned Julia deployment assembly
3. keep current Julia Arrow IPC operability intact

Completion conditions:

1. Julia plugin package owns launch metadata, capability declarations, and
   deployment artifacts
2. host consumes Julia through plugin contracts rather than direct ownership
3. current rerank/analyzer flows retain parity

#### Phase M5: Generic Artifact and Endpoint Cutover

Purpose:

1. complete the cutover from Julia-specific outward surfaces to generic plugin
   artifact surfaces
2. reduce legacy Julia-named host exports to explicit compatibility shims only

Completion conditions:

1. generic plugin-artifact endpoints are canonical
2. Julia-named outward surfaces are compatibility-only
3. Studio, OpenAPI, and Zhenfa surfaces all point to generic contracts first

#### Phase M6: Additional Plugin Onboarding Readiness

Purpose:

1. prove that a second language plugin can be onboarded without core expansion
2. validate that the architecture is genuinely additive

Completion conditions:

1. one non-Julia plugin path can be introduced using the new contracts
2. no new language-specific host expansion is required

### 8.2 Current Program Position

The current tree has completed **Phase M6** and is now at the handoff to
**Phase 7**.

What is already true:

1. `xiuxian-wendao-core` and `xiuxian-wendao-runtime` both exist as physical
   crates in the workspace
2. Julia-specific ownership is package-owned in `xiuxian-wendao-julia`
3. generic plugin-artifact outward surfaces are canonical and the host
   compatibility shims are retired from the live crate root
4. one non-Julia plugin path (`xiuxian-wendao-modelica`) already proves that
   repo-facing, docs-facing, and Studio-facing consumers can expand without
   new language-specific host structs
5. the active migration risk is no longer extraction ambiguity or additive-
   proof fragmentation; it is transport/runtime hardening ambiguity at the
   Phase-7 opening boundary

What is not yet complete:

1. the next macro-phase proposal still needs to be executed after the
   late-`M6` additive-proof track has been formally signed off

### 8.3 Anti-Fragmentation Rule

From this point forward, implementation should not be organized as isolated
micro-refactors without explicit attachment to one macro phase.

Every code task must answer:

1. which macro phase it belongs to
2. which phase gate it advances
3. which ownership boundary it changes
4. which compatibility seam it retires, preserves, or narrows

Work that cannot answer those questions should be treated as out of program.

### 8.4 Program-Level Stop Conditions

Pause the migration program if any of the following becomes true:

1. a new language-specific host type lands outside a compatibility seam
2. crate extraction starts before ownership and namespace cleanup are physically
   complete
3. Julia parity regresses in runtime behavior or deployment operability
4. compatibility shims begin receiving new implementation logic
5. feature-folder discipline is bypassed in touched medium or complex seams

The directory sketch above is normative in style, not just illustrative:

1. namespace layout must follow functional ownership
2. medium or complex features must land as folders with focused leaf modules
3. crate roots and feature roots must not become logic sinks

## 7.3 Structural Acceptance Rules

The following structural rules are mandatory for this migration.

### Create a feature folder when:

1. a slice owns multiple concerns such as types, orchestration, transport, parsing, or tests
2. a feature has more than one stable public concept
3. a feature is expected to evolve independently across phases
4. a file would otherwise mix contracts, orchestration, and helper logic

### A leaf file is acceptable when:

1. the responsibility is singular and stable
2. the file does not act as a catch-all sink
3. the file does not need multiple child namespaces to preserve clarity

### Split a file further when:

1. it owns unrelated concerns
2. it becomes a DTO or type warehouse
3. it mixes transport, orchestration, and contract logic
4. tests can no longer mirror the internal ownership cleanly

### Public re-exports must be stabilized by:

1. keeping `mod.rs` interface-only
2. re-exporting existing public names from the feature root where compatibility matters
3. moving implementation logic behind responsibility-oriented leaf modules

### Test layout must mirror feature layout:

1. medium or complex features should keep tests in the same feature folder or a mirrored test namespace
2. tests should follow feature seams such as `capabilities/`, `artifacts/`, `transport/`, and `launch/`
3. migration phases should not leave test topology flatter and less expressive than production topology

## 8. Plugin Protocol Boundary

## 8.1 Manifest

Every plugin package should declare:

1. plugin id
2. plugin version
3. host API version
4. runtime kind
5. capabilities
6. supported transports
7. artifact declarations
8. compatibility constraints

Illustrative shape:

```toml
id = "wendao-julia"
version = "0.1.0"
api_version = "v1"
runtime = "julia"

[[capabilities]]
id = "rerank"
contract_version = "v1"
transports = ["flight", "arrow_ipc_http"]

[[artifacts]]
id = "deployment"
formats = ["toml", "json"]
```

## 8.2 Capability Contracts

Each capability should declare:

1. capability id
2. contract version
3. input schema
4. output schema
5. execution mode
6. transport support
7. optional pushdown support

## 8.3 Artifact Contracts

Artifacts are plugin-owned outputs surfaced through a host-standardized inspection boundary.

The host should expose a generic artifact model rather than hardcoding language-specific artifact types.

## 9. What Moves Out of the Current `xiuxian-wendao`

The following categories must migrate toward `runtime` or plugin packages:

1. language-specific runtime config
2. language-specific deployment artifact structs
3. language-specific OpenAPI/UI response wrappers
4. source inclusion of sibling plugin crates through `#[path]`
5. builtin registration that assumes plugin code must compile into core

The following categories belong in `core`:

1. shared repository-intelligence contracts
2. normalized records
3. schema versioning
4. DataFusion-oriented capability abstractions
5. transport-neutral capability and artifact descriptors

## 10. Julia Migration Rules

Julia must remain first-class during migration.

That requires:

1. no regression in current Arrow IPC-based rerank or analyzer paths
2. no forced Rust-native ABI detour
3. no host-side rewrite that makes Julia-specific transport details permanent core API

The migration stance is:

1. preserve current Julia functionality
2. lift Julia-specific host types into generic plugin abstractions
3. move Julia-specific ownership into `xiuxian-wendao-julia`

## 11. Phase Plan

## Phase 0: Contract Freeze and Mapping

Objectives:

1. inventory current Julia-specific host surfaces
2. map each surface to target `core`, `runtime`, or plugin ownership
3. freeze naming for capability, transport, and artifact concepts

Exit criteria:

1. migration map is documented
2. no new language-specific core types are introduced

## Phase 1: Logical Boundary Extraction In Place

Objectives:

1. extract capability, artifact, and transport concepts inside the current crate
2. stop adding new Julia-specific runtime surfaces
3. introduce generic plugin artifact and runtime config models
4. land the new abstractions in feature-folder form rather than new flat files

Exit criteria:

1. new generic host-side types exist
2. old Julia-specific types are only compatibility wrappers or aliases
3. touched boundaries are split by responsibility and keep `mod.rs` interface-only

## Phase 2: Introduce `xiuxian-wendao-core`

Objectives:

1. move stable contracts into a new crate
2. keep public compatibility through re-exports where needed
3. add semver governance for core API

Exit criteria:

1. `core` builds independently
2. plugin packages depend on `core` rather than the whole host crate
3. `core` namespaces are responsibility-oriented and do not regress into flat contract warehouses

## Phase 3: Introduce `xiuxian-wendao-runtime`

Objectives:

1. move runtime assembly and negotiation out of the monolith
2. centralize discovery, lifecycle, and transport fallback
3. keep existing binaries functional through delegation

Exit criteria:

1. `runtime` owns transport negotiation and lifecycle
2. host binaries can delegate to `runtime`
3. runtime extraction does not create new orchestration monoliths or implementation-heavy `mod.rs`

## Phase 4: Externalize Julia Ownership

Objectives:

1. make `xiuxian-wendao-julia` the owner of Julia capabilities and artifacts
2. remove `#[path]` source inclusion
3. use package-level manifests and runtime registration

Exit criteria:

1. Julia plugin compiles and publishes independently
2. host consumes Julia through package metadata and runtime wiring
3. Julia package layout uses capability, artifact, and launch folders rather than a crate-root implementation sink

## Phase 5: Generic Artifact and UI Migration

Objectives:

1. replace language-specific host artifact surfaces with generic plugin artifact endpoints
2. update Studio and gateway contracts to query plugin artifacts generically

Exit criteria:

1. no Julia-specific artifact endpoint is required in core
2. plugin artifacts are surfaced by plugin id and artifact id
3. generic artifact and UI surfaces preserve namespace clarity instead of introducing new mixed DTO/controller files

## Phase 6: Additional Plugin Onboarding Readiness

Objectives:

1. onboard at least one non-Julia plugin path without core expansion
2. prove the architecture is capability-first rather than Julia-specialized
3. turn the additive proof into a governed program checkpoint rather than a
   stream of detached endpoint slices

Exit criteria:

1. at least one non-Julia plugin can land without new language-specific host
   structs
2. repo-facing, docs-facing, and Studio-facing consumers all have bounded
   additive proof coverage
3. the RFC, ExecPlan, and Wendao program note agree on the current macro-phase
   position and the next staged push plan

## Phase 7: Flight-First Runtime Negotiation

Objectives:

1. harden transport preference order after additive plugin proof is closed
2. make Flight the preferred transport where capability providers can support
   it
3. preserve Arrow IPC fallback while keeping transport decisions observable

Exit criteria:

1. runtime diagnostics expose negotiated transport
2. fallback decisions are explicit and observable
3. transport hardening does not reintroduce language-specific host behavior

### 11.3 Phase-7 Staged Push Plan

The `Phase 7` rollout should now proceed as a bounded transport-hardening
program rather than as scattered runtime tweaks.

Current phase status:

1. `Phase 7` is open
2. `Phase-7 Stage A` is complete: the transport-surface inventory now names
   the live contract, runtime, and outward inspection seams
3. `Phase-7 Stage B` is now in progress: the runtime-owned negotiation policy
   seam has landed and the rerank path now delegates through it
4. `Phase-7 Stage C` remains pending behind outward diagnostics and the final
   transport gate

#### Phase-7 Stage A: Transport Surface Inventory Bundle

Objectives:

1. identify the live transport-negotiation seams, fallback callers, and
   outward diagnostics surfaces
2. declare one canonical transport preference order per capability family
3. record where negotiated transport and fallback reason should be surfaced

Exit criteria:

1. the RFC, program note, and active ExecPlan all name the same touched
   negotiation and diagnostics seams
2. the preferred transport order is documented without ambiguity
3. the next implementation slice can cite one bounded transport ownership seam

Stage-A inventory findings:

1. the generic transport contract surface already lives in
   `xiuxian-wendao-core` through `PluginCapabilityBinding`,
   `PluginTransportEndpoint`, and `PluginTransportKind`; the currently
   declared transport kinds are `ArrowFlight`, `ArrowIpcHttp`, and
   `LocalProcessArrowIpc`
2. the only live runtime-owned client-construction seam today is
   `xiuxian-wendao-runtime/src/transport/client.rs`, where
   `build_arrow_transport_client_from_binding(...)` materializes Arrow IPC
   over HTTP from a generic capability binding; there is not yet a runtime-
   owned multi-transport selector in the live tree
3. the host-side `src/gateway/studio/search/handlers/arrow_transport.rs` seam
   is a local Arrow response encoder, not a plugin transport-negotiation owner
4. the current outward inspection surface for transport metadata is
   `UiPluginArtifact` in `src/gateway/studio/types/config.rs`, which already
   exposes `base_url`, `route`, `health_route`, `timeout_secs`, and
   `schema_version`
5. the canonical Phase-7 preference order is now fixed as
   `ArrowFlight -> ArrowIpcHttp -> LocalProcessArrowIpc`; `ArrowIpcHttp`
   remains the bounded compatibility fallback, and `LocalProcessArrowIpc`
   stays reserved for explicitly managed local-provider paths
6. `Phase-7 Stage B` should therefore extend the runtime-owned
   `xiuxian-wendao-runtime/src/transport/` seam instead of introducing new
   transport-selection logic under host gateway handlers

#### Phase-7 Stage B: Negotiation Policy Bundle

Objectives:

1. harden runtime negotiation so Flight is preferred where providers support
   it
2. preserve Arrow IPC fallback as the bounded compatibility path
3. keep provider-specific transport detail out of generic host vocabulary

Exit criteria:

1. the runtime selects Flight first on the targeted capability path
2. fallback remains explicit and deterministic
3. touched runtime tests prove both preferred and fallback paths

Current Stage-B status:

1. the runtime-owned negotiation policy now lives in
   `xiuxian-wendao-runtime/src/transport/negotiation.rs`
2. the canonical preference order is now executable in code, not only
   documented in Stage-A notes
3. the rerank transport path now delegates through
   `negotiate_arrow_transport_client_from_bindings(...)` instead of directly
   constructing an Arrow IPC client from the raw binding
4. the runtime now also owns a real Flight client materialization seam in
   `xiuxian-wendao-runtime/src/transport/flight.rs`
5. that Flight client intentionally rides the LanceDB-owned Arrow `57.3`
   line through `arrow-flight = 57.3.0`, then uses the existing
   `xiuxian-vector` `58 -> 57.3 -> 58` batch bridge to keep the host-side
   rerank path on the current engine Arrow line
6. fallback from an incomplete configured `ArrowFlight` binding to a lower-
   preference `ArrowIpcHttp` binding is now deterministic and covered by
   runtime tests, while a supported `ArrowFlight` binding is now selected
   first and can process a real roundtrip against a mock Flight service
7. `Phase-7 Stage B` is now complete; the governed next move is
   `Phase-7 Stage C: Observability and Gate Bundle`

#### Phase-7 Stage C: Observability and Gate Bundle

Objectives:

1. expose negotiated transport and fallback reason through runtime diagnostics
   and outward inspection surfaces
2. verify that transport hardening did not regress the additive plugin path
3. record an explicit `Phase 7` gate decision before opening the next phase

Exit criteria:

1. runtime diagnostics expose negotiated transport
2. outward gateway/tool/debug surfaces can report fallback decisions
3. the RFC records a go/no-go decision for `Phase 7`

### 11.1 Late-M6 Staged Push Plan

The next RFC-governed push should be executed as three bounded stages, not as
unrelated endpoint picks.

Current stage status:

1. `Stage A` is complete: the external Modelica path now covers the remaining
   Studio repo service-state bundle, including `/api/repo/index` and
   `/api/repo/index/status`
2. `Stage B` is complete: the active RFC, program note, route inventory, and
   package note now describe the same late-`M6` outward position
3. `Stage C` is complete: the `M6` exit review now records a go decision and
   opens the next macro-phase target

#### Stage A: Gateway Completion Bundle

Objectives:

1. close the remaining Studio repo service-state family as one bundle
2. keep external-plugin proofs grouped by outward family instead of single
   route drift
3. consolidate local selector/config helpers only where they reduce repeated
   proof wiring

Exit criteria:

1. the remaining live Studio repo family, led by `/api/repo/index/status`, is
   covered by the external Modelica path, including the sibling
   `/api/repo/index` enqueue route
2. helper consolidation does not introduce new dead-code suppressions
3. grouped route verification stays green for the touched family

#### Stage B: Outward Contract Alignment Bundle

Objectives:

1. align the RFC, program note, route inventory, and outward contract notes
   with the now-broadened late-`M6` proof set
2. remove stale early-phase claims from active status sections
3. keep the additive proof discoverable from one Wendao-local entrypoint

Exit criteria:

1. the active RFC/program note no longer describe the tree as early-phase
   extraction work
2. outward inventory and package-roadmap notes describe the same late-`M6`
   position
3. the staged push plan is recorded in the active ExecPlan and GTD log

#### Stage C: M6 Exit Review

Objectives:

1. compare the accumulated Modelica proof set against the `M6` completion
   conditions
2. decide whether `M6` is complete or whether one more bounded bundle is
   required
3. open the next macro-phase proposal only after the late-`M6` proof is
   explicitly signed off

Exit criteria:

1. the RFC records a go/no-go decision for `M6` completion
2. the program note records the next macro-phase target instead of leaving the
   program in open-ended additive drift
3. follow-up implementation work can cite one stage and one gate before
   reading more code

### 11.2 M6 Exit Review Decision

Decision: `go`

The late-`M6` additive-proof track is complete.

Criteria review:

1. `M6` criterion 1 is satisfied:
   `xiuxian-wendao-modelica` lands as one non-Julia plugin path without new
   language-specific host structs.
2. `M6` criterion 2 is satisfied:
   repo-facing, docs-facing, and Studio-facing consumers all now have bounded
   additive proof coverage, including the final Studio repo service-state
   bundle `/api/repo/index` and `/api/repo/index/status`.
3. `M6` criterion 3 is satisfied:
   the RFC, active ExecPlan, Wendao program note, outward inventory, and
   Modelica package note now agree on the same late-`M6` position and on the
   next governed move.

Next macro-phase target:

1. open `Phase 7: Flight-First Runtime Negotiation`
2. treat transport preference hardening and transport observability as the
   next governed program concern
3. stop accumulating additive-proof slices unless a later phase explicitly
   requires them

## 12. Governance and Tooling

The migration should be backed by ecosystem tooling:

1. `cargo-deny` for advisory, license, and duplicate-dependency policy
2. `cargo-machete` and `cargo-udeps` for dependency hygiene
3. `cargo-semver-checks` for `core` contract stability
4. `guppy` and `cargo-hakari` for workspace dependency governance
5. `cargo-dist` for Rust-side runtime and packaging distribution where applicable

These tools improve release hygiene and structure, but they do not replace the need for a host-defined plugin protocol.

## 14. Structural Migration Defaults

Unless a narrower slice is genuinely trivial, implementation should default to:

1. `feature_name/mod.rs` plus leaf modules instead of expanding `feature_name.rs`
2. splitting by responsibility before moving logic across crates
3. mirroring runtime and plugin feature seams in tests
4. preserving stable public exports while changing internal physical layout

These defaults are mandatory for migration phases unless a documented exception is approved at the RFC or blueprint level.

## 15. Risks

### Risk 1: Premature Physical Split

If crates are split before boundaries are clean, the same confusion will simply be spread across more crates.

Mitigation:

1. do logical boundary extraction before physical crate extraction
2. require feature-folder-first modularization before phase completion

### Risk 2: Julia Regression

If host abstractions change faster than Julia ownership migration, current rerank and analyzer paths may break.

Mitigation:

1. compatibility wrappers
2. explicit Julia migration phase
3. no removal before parity

### Risk 3: Runtime/Core Leakage

If `runtime` starts exporting unstable lifecycle types as if they were core API, the split loses meaning.

Mitigation:

1. strict ownership rules
2. semver checks on `core`
3. structural acceptance rules enforced during migration gates

### Risk 4: Flat Modularization Theater

If migration creates many files but keeps mixed responsibility and poor namespace naming, the architecture will remain unclear despite the physical split.

Mitigation:

1. hard structural acceptance rules
2. responsibility-oriented naming requirements
3. test topology mirroring feature topology

## 16. Decision

Wendao should migrate to a layered architecture with:

1. `xiuxian-wendao-core` for stable contracts
2. `xiuxian-wendao-runtime` for host behavior
3. independently published plugin packages for language-native capability ownership
4. a compatibility bridge while the migration is underway

This migration should proceed in staged phases, not as a one-shot crate explosion, and every phase must satisfy feature-folder-first modularization rules before it can be considered complete.
