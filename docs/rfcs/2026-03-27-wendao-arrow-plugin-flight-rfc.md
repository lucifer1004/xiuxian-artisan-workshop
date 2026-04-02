---
type: knowledge
title: "RFC: Wendao Arrow-First Plugin Protocol with Flight-First Transport"
category: "rfc"
status: "draft"
authors:
  - codex
created: 2026-03-27
tags:
  - rfc
  - wendao
  - arrow
  - datafusion
  - plugins
  - flight
  - julia
metadata:
  title: "RFC: Wendao Arrow-First Plugin Protocol with Flight-First Transport"
---

# RFC: Wendao Arrow-First Plugin Protocol with Flight-First Transport

## 1. Summary

This RFC proposes a **protocol-first plugin architecture** for Wendao in which:

1. `Arrow` is the canonical plugin data plane.
2. `DataFusion` is the host-side execution kernel.
3. `Arrow Flight` is the preferred plugin transport.
4. `Arrow IPC` remains the required fallback transport for ecosystems that do not yet expose Flight support in their current Arrow implementations.

This RFC rejects a Rust-trait-centric plugin architecture as the primary extension surface for Wendao. The correct stable boundary is a language-neutral Arrow contract, not a Rust ABI or a workspace-local registration trick.

## 1.1 Boundary Override (2026-04-01)

By explicit operator direction, the active architectural target is now stricter
than this RFC's original `Flight-first + Arrow IPC fallback` stance.

The boundary override is:

1. stable interactive Wendao query, retrieval, repo, docs, planner, status,
   and config surfaces should target high-performance-first `Arrow Flight` as
   the desired external contract
2. `JSON` remains the control surface for process liveness/bootstrap,
   operator config/status/control, and static manifest or artifact inspection
   until a dedicated Flight control-plane replacement lands
3. `ArrowIpcHttp` and local-process IPC remain transitional compatibility debt,
   but are not part of the formal target boundary
4. no new long-term Wendao business surface should be justified as JSON-first
   once this override is active

This is a governed override to the older fallback stance, not an accidental
drift. Implementation should therefore classify current surfaces into two
formal classes, `Arrow Flight` business surface and `JSON` control surface,
while tracking current IPC paths only as migration debt rather than
continuing to expand them as if they were stable target contracts.

## 2. Motivation

Wendao already has a clear technical center of gravity:

1. `Arrow IPC` and `RecordBatch` are the natural data-exchange boundary.
2. `DataFusion` is becoming the query and execution kernel.
3. Language-native ecosystems such as Julia, Python, and JavaScript already have strong Arrow support.
4. Heavy retrieval and reranking paths benefit from staying columnar and avoiding bespoke bindings.

The current Julia direction confirms the shape of the intended architecture rather than contradicting it:

1. `.data/WendaoArrow` and `.data/WendaoAnalyzer` can provide high-performance rerank and analyzer services.
2. Julia can exchange Arrow batches directly without Rust-specific bindings.
3. The same model can later extend to Python and JavaScript through their own Arrow runtimes.

The unresolved design question is therefore not whether Wendao should have plugins, but:

**How should Wendao formalize a plugin protocol so Arrow/DataFusion remain first-class while Flight adoption can advance without breaking ecosystems that still need Arrow IPC fallback?**

## 3. Problem Statement

The current implementation has two competing architectural tendencies:

1. The data plane is already moving toward Arrow-native, language-native services.
2. The core runtime still contains language-specific types and endpoints such as Julia rerank runtime config and Julia deployment artifact surfaces.

If this continues, Wendao will drift into a poor middle ground:

1. The host core will accumulate language-specific config structs and artifact types.
2. New ecosystems such as Python or JavaScript will force repeated copies of the Julia path.
3. DataFusion integration will remain ad hoc instead of becoming the organizing execution model.

The architecture must instead make Arrow and capability contracts central, while leaving language and transport details to plugin packages.

## 4. Goals

This RFC has the following goals:

1. Define Wendao plugins as **language-native services** that communicate through Arrow-native contracts.
2. State that `DataFusion` is the host-side execution kernel for plugin integration.
3. Make `Arrow Flight` the preferred transport for new plugin development.
4. Preserve `Arrow IPC` fallback for runtimes whose Arrow libraries do not yet support Flight.
5. Prevent Julia-specific or language-specific transport and artifact types from leaking into core runtime contracts.
6. Keep plugin packages independently publishable and independently evolvable.

## 5. Non-Goals

This RFC does not attempt to:

1. Replace DataFusion with a custom execution engine.
2. Standardize every future plugin capability in one pass.
3. Require Flight support before a plugin can exist.
4. Mandate one plugin runtime language.
5. Land the full implementation in this document.

## 6. Core Assumptions

### 6.1 Arrow Is the Data Plane

For Wendao plugins, Arrow is not an optimization layer. It is the canonical data plane.

That means:

1. High-value plugin capabilities consume Arrow inputs.
2. High-value plugin capabilities produce Arrow outputs.
3. JSON and TOML remain control-plane formats, not the main result path for search, rerank, analysis, or feature extraction.

### 6.2 DataFusion Is the Host Execution Kernel

The Wendao host must treat plugin capabilities as DataFusion-adjacent execution units rather than arbitrary RPC helpers.

In practice:

1. plugin inputs should be representable as `RecordBatch` or `RecordBatchStream`
2. plugin outputs should be consumable by host-side execution plans
3. pushdown, projection, and bounded materialization decisions should remain aligned with DataFusion execution semantics

### 6.3 Plugins Are Packages and Services, Not Linked Rust Modules

The stable plugin unit is:

1. a plugin package
2. a manifest
3. one or more capabilities
4. one or more transport endpoints

This RFC therefore rejects a design where the primary extension story is:

1. compile a Rust crate into the Wendao workspace
2. register it through a Rust trait
3. let the core link language-specific behavior directly

That model is acceptable for internal testing or bootstrap paths, but it is not the ecosystem architecture.

## 7. Why Flight First

`Arrow Flight` should be the preferred transport because it aligns best with the long-term Wendao shape.

### 7.1 Strongest alignment with Arrow-native remote execution

Flight preserves the Arrow-centric execution model and is more natural for:

1. streaming batches
2. schema negotiation
3. service discovery
4. future remote execution growth

### 7.2 Better long-term fit for DataFusion-oriented plugin execution

If Wendao continues to become more operator-first and execution-plan-aware, Flight is a cleaner long-term transport for:

1. batch streams
2. backpressure-aware flows
3. partitioned processing
4. remote service composition

### 7.3 Cleaner multi-language ecosystem story

A Flight-first story avoids turning each language plugin into a custom HTTP/JSON service with Arrow attached as an afterthought.

## 8. Current Julia Constraint

The Julia Arrow package direction is strong for Arrow IPC but, in the current state discussed for this RFC, it does not yet provide:

1. tensors or sparse tensors
2. Flight RPC
3. the C data interface

For Wendao, the important part is not the missing tensor surface. The architectural pressure point is the lack of `Flight RPC`.

This means:

1. Julia should not block the Flight-first architecture.
2. Wendao should not force Julia through a Rust-native ABI path.
3. Wendao must specify a sanctioned fallback transport so Julia remains a first-class plugin runtime today.

## 9. Proposed Architecture

## 9.1 Two-Plane Model

### Control Plane

The control plane is host-owned and metadata-oriented.

It covers:

1. plugin discovery
2. manifest loading
3. capability registration
4. version negotiation
5. lifecycle and health
6. configuration injection
7. artifact discovery
8. transport selection

### Data Plane

The data plane is Arrow-native and capability-oriented.

It covers:

1. request and response schemas
2. record-batch exchange
3. stream exchange where supported
4. execution metrics and trace metadata

## 9.2 Capability-First, Not Language-First

The host core should route by capability, not by language.

Examples:

1. `rerank.v1`
2. `analyze_repository.v1`
3. `feature_extract.v1`
4. `artifact_export.v1`

Languages only describe who implements a capability, not how the host core models the capability.

## 9.3 Plugin Package Shape

Each plugin package should publish metadata similar to:

```toml
id = "wendao-julia"
version = "0.1.0"
api_version = "v1"
runtime = "julia"

[[capabilities]]
id = "rerank"
contract_version = "v1"
transport_priority = ["flight", "arrow_ipc_http", "arrow_ipc_process"]

[[capabilities]]
id = "analyze_repository"
contract_version = "v1"
transport_priority = ["arrow_ipc_http", "arrow_ipc_process"]

[[artifacts]]
id = "deployment"
formats = ["toml", "json"]
```

The exact manifest syntax can evolve, but the architectural rules should not.

## 10. Transport Selection Policy

The host must select transport through an explicit ordered policy rather than hardcoding one transport per language.

## 10.1 Priority Order

For capability execution, the default order should be:

1. `Arrow Flight`
2. `Arrow IPC over HTTP`
3. `local process Arrow IPC`

This order expresses the long-term preference without excluding current ecosystems.

## 10.2 Requirements

### Flight

Use `Flight` when:

1. the plugin runtime exposes Flight support
2. the capability declares Flight compatibility
3. host and plugin agree on the contract version

### Arrow IPC over HTTP

Use Arrow IPC over HTTP when:

1. Flight is not implemented in the plugin runtime
2. the plugin package exposes a stable Arrow IPC service endpoint
3. the capability remains batch-oriented and schema-stable

This is the expected current Julia fallback.

### Local Process Arrow IPC

Use local process Arrow IPC only when:

1. the plugin is host-local
2. service startup and lifecycle are also managed locally
3. the plugin does not yet provide a stable network endpoint

This should remain a secondary operational mode rather than the default public contract.

## 11. DataFusion Integration Requirements

Wendao should not treat plugin calls as opaque sidecars. They must integrate with the execution kernel.

At minimum, the host integration model should support:

1. Arrow batch input and output
2. schema validation before execution
3. projection-aware request shaping
4. bounded materialization
5. trace metadata propagation
6. future binding into DataFusion execution plans or adapters

The important principle is:

**plugin capabilities should feel like Arrow-native operators from the host perspective, even when they run in Julia, Python, or JavaScript.**

## 12. Artifact Model

The current Julia deployment artifact points at a useful pattern, but the host core should generalize it.

Instead of core-owned types such as:

1. `JuliaRerankRuntimeConfig`
2. `JuliaDeploymentArtifact`
3. `UiJuliaDeploymentArtifact`

the host should move toward:

1. `PluginCapabilityRuntimeConfig`
2. `PluginArtifactDescriptor`
3. `PluginArtifactPayload`

Then the Julia plugin provides:

1. plugin id: `wendao-julia`
2. artifact id: `deployment`
3. artifact formats: `toml`, `json`

This keeps artifact ownership with the plugin while keeping artifact transport and inspection host-standardized.

## 13. Configuration Direction

The host configuration should not keep growing language-named fields such as `julia_rerank`.

The target shape should instead be capability-oriented. For example:

```toml
[link_graph.retrieval.reranker]
provider = "wendao-julia"
capability = "rerank"
contract_version = "v1"
preferred_transport = "flight"

[link_graph.retrieval.reranker.options]
base_url = "http://127.0.0.1:18080"
route = "/arrow-ipc"
schema_version = "v1"
service_mode = "stream"
```

If the plugin runtime later supports Flight, the provider and capability remain stable while transport selection can change.

## 14. Fallback Semantics

The fallback policy must be explicit and observable.

When Flight is preferred but unavailable, the host should:

1. record the reason the preferred transport was not selected
2. select the next compatible transport
3. expose the chosen transport in diagnostics and deployment artifacts
4. refuse execution only when no compatible transport remains

This avoids silent downgrade while preserving operational continuity.

## 15. Consequences

### Positive

1. Wendao keeps Arrow and DataFusion as first-class architecture, not just implementation details.
2. Julia remains a high-performance first-class runtime today through Arrow IPC fallback.
3. Python and JavaScript can join the same architecture without bindings-heavy host work.
4. Plugin packages become independently publishable and independently evolvable.
5. Core types stop growing language-specific transport and artifact surfaces.

### Negative

1. The plugin manifest and capability contract surface become more formal.
2. Schema governance and versioning discipline become mandatory.
3. The host must own transport selection logic rather than hiding it inside language-specific codepaths.

## 16. Recommended Migration Direction

The next steps should be:

1. define a plugin manifest for capability and transport declaration
2. generalize the Julia deployment artifact into a plugin artifact model
3. replace language-named host config such as `julia_rerank` with provider and capability selection
4. keep Julia on Arrow IPC fallback until its Arrow ecosystem can expose Flight support
5. add Flight-first design hooks now so future runtimes do not repeat the Julia-specific path

## 17. Open Questions

1. Should Wendao adopt `Arrow Flight` first or `Flight SQL` first for remote plugin interaction?
2. Which capability classes must be stream-capable in the first transport contract, and which can remain batch-only?
3. How much DataFusion pushdown information should be expressible in the first plugin contract?
4. Should plugin artifacts be returned as Arrow-backed metadata tables, JSON/TOML payloads, or both?

## 18. Decision

Wendao should standardize on an **Arrow-first plugin protocol** with **Flight-first transport policy** and **Arrow IPC fallback**.

For the present Julia ecosystem state, the correct posture is:

1. keep Julia as a first-class plugin runtime
2. keep Arrow IPC as the sanctioned Julia fallback
3. do not let Julia-specific transport constraints leak into core architecture
4. design the host now for a future where Flight becomes available
