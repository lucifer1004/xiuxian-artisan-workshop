# Wendao Package Layering

:PROPERTIES:
:ID: wendao-package-layering
:PARENT: [[index]]
:TAGS: architecture, core, runtime, plugins, layering
:STATUS: ACTIVE
:END:

## Purpose

Define the architectural ownership boundary among:

1. `xiuxian-wendao-core`
2. `xiuxian-wendao-runtime`
3. `xiuxian-wendao`

This note is the architectural rule for new code placement. It is not a claim
that the current tree has already completed the migration.

## Layer Definitions

### `xiuxian-wendao-core`

`core` is the stable contract kernel.

It owns:

1. ids and selectors
2. stable request or response record shapes
3. capability, artifact, and transport descriptors
4. plugin-facing traits and contract enums
5. schema and route constants that are contract, not execution

It must not own:

1. filesystem or env resolution
2. Flight/DataFusion client or server execution
3. graph or retrieval algorithms
4. parser implementation
5. `xiuxian-vector`-backed execution logic

### `xiuxian-wendao-runtime`

`runtime` is the host execution kernel.

It owns:

1. config and settings resolution
2. transport negotiation
3. Arrow Flight client and server wiring
4. DataFusion session bootstrap and runtime query execution glue
5. request metadata decoding and contract materialization
6. plugin registry, loading, and host-side orchestration

It must not own:

1. stable contract ownership that plugins consume directly
2. Wendao graph semantics
3. Wendao retrieval semantics
4. plugin-specific thick implementation

### `xiuxian-wendao`

`wendao` is the domain kernel.

It owns:

1. `link_graph`
2. graph algorithms, traversal, PPR, saliency, and relation semantics
3. parser implementation for general Wendao document or code understanding
4. search, retrieval, fusion, and storage semantics
5. `xiuxian-vector`-backed domain retrieval behavior
6. business-domain services and transitional compatibility seams

It must not become the long-term owner of:

1. new stable plugin contracts
2. new generic runtime helpers
3. plugin-specific thick implementation that can live in its own crate

## Core Rule

Do not classify code by how important it feels.

Classify it by which kind of ownership it requires:

1. stable contract ownership -> `core`
2. host runtime ownership -> `runtime`
3. Wendao domain ownership -> `wendao`

## Data Layer Interpretation

The same data-plane stack splits across layers.

### Arrow Flight

- Flight contract records and route constants -> `core`
- Flight server or client execution and negotiation -> `runtime`
- Flight-backed business semantics -> `wendao` or a plugin crate

### DataFusion

- query contract shape -> `core`
- session bootstrap and query execution glue -> `runtime`
- Wendao query semantics and business planning -> `wendao`

### `xiuxian-vector`

If a component depends on `xiuxian-vector` to execute retrieval semantics, it
is no longer a pure contract.

That code belongs in:

1. `wendao` when it is domain retrieval logic
2. `runtime` when it is generic host wiring

It does not belong in `core`.

## Link Graph And Parser Rule

`link_graph` and the general Wendao parser stack are domain core, not contract
core.

They belong in `xiuxian-wendao` because they define how Wendao understands and
retrieves knowledge.

Only their stable plugin-facing contracts should move to `core`.

## Gateway Rule

`gateway` is an adapter layer, not the primary home of domain behavior.

Its long-term role is:

1. decode protocol input
2. validate contract metadata
3. dispatch into runtime or domain services
4. encode protocol output

Therefore:

- thin Arrow Flight/DataFusion contract dispatch is acceptable at the gateway
  boundary
- thick search, graph, parser, and plugin business logic should live below the
  gateway boundary

## Plugin Rule

A plugin crate should own as much plugin-specific implementation as possible.

The host should prefer:

1. add dependency
2. register capability
3. compile and load plugin

The host should avoid:

1. adding new plugin-specific business modules to `xiuxian-wendao`
2. hard-coding plugin-specific parser or launch behavior in the host crate

:RELATIONS:
:LINKS: [[index]], [[06_roadmap/412_core_runtime_plugin_program]], [[06_roadmap/415_m4_julia_externalization_package_list]], [[06_roadmap/417_wendao_package_boundary_matrix]]
:END:

---

:FOOTER:
:AUDITOR: codex
:END:
