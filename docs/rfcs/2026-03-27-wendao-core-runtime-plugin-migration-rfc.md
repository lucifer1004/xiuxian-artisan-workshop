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

Exit criteria:

1. new generic host-side types exist
2. old Julia-specific types are only compatibility wrappers or aliases

## Phase 2: Introduce `xiuxian-wendao-core`

Objectives:

1. move stable contracts into a new crate
2. keep public compatibility through re-exports where needed
3. add semver governance for core API

Exit criteria:

1. `core` builds independently
2. plugin packages depend on `core` rather than the whole host crate

## Phase 3: Introduce `xiuxian-wendao-runtime`

Objectives:

1. move runtime assembly and negotiation out of the monolith
2. centralize discovery, lifecycle, and transport fallback
3. keep existing binaries functional through delegation

Exit criteria:

1. `runtime` owns transport negotiation and lifecycle
2. host binaries can delegate to `runtime`

## Phase 4: Externalize Julia Ownership

Objectives:

1. make `xiuxian-wendao-julia` the owner of Julia capabilities and artifacts
2. remove `#[path]` source inclusion
3. use package-level manifests and runtime registration

Exit criteria:

1. Julia plugin compiles and publishes independently
2. host consumes Julia through package metadata and runtime wiring

## Phase 5: Generic Artifact and UI Migration

Objectives:

1. replace language-specific host artifact surfaces with generic plugin artifact endpoints
2. update Studio and gateway contracts to query plugin artifacts generically

Exit criteria:

1. no Julia-specific artifact endpoint is required in core
2. plugin artifacts are surfaced by plugin id and artifact id

## Phase 6: Flight-First Runtime Negotiation

Objectives:

1. implement transport preference order
2. make Flight the preferred transport
3. preserve Arrow IPC fallback

Exit criteria:

1. runtime diagnostics expose negotiated transport
2. fallback decisions are explicit and observable

## Phase 7: Additional Language Plugins

Objectives:

1. onboard Python and JavaScript without core expansion
2. prove the architecture is capability-first rather than Julia-specialized

Exit criteria:

1. at least one non-Julia plugin can land without new language-specific host structs

## 12. Governance and Tooling

The migration should be backed by ecosystem tooling:

1. `cargo-deny` for advisory, license, and duplicate-dependency policy
2. `cargo-machete` and `cargo-udeps` for dependency hygiene
3. `cargo-semver-checks` for `core` contract stability
4. `guppy` and `cargo-hakari` for workspace dependency governance
5. `cargo-dist` for Rust-side runtime and packaging distribution where applicable

These tools improve release hygiene and structure, but they do not replace the need for a host-defined plugin protocol.

## 13. Risks

### Risk 1: Premature Physical Split

If crates are split before boundaries are clean, the same confusion will simply be spread across more crates.

Mitigation:

1. do logical boundary extraction before physical crate extraction

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

## 14. Decision

Wendao should migrate to a layered architecture with:

1. `xiuxian-wendao-core` for stable contracts
2. `xiuxian-wendao-runtime` for host behavior
3. independently published plugin packages for language-native capability ownership
4. a compatibility bridge while the migration is underway

This migration should proceed in staged phases, not as a one-shot crate explosion.
