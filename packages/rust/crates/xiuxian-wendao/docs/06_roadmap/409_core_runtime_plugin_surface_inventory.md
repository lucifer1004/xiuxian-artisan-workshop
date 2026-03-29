# Wendao Core Runtime Plugin Surface Inventory

:PROPERTIES:
:ID: wendao-core-runtime-plugin-surface-inventory
:PARENT: [[index]]
:TAGS: roadmap, migration, plugins, core, runtime, julia, inventory
:STATUS: ACTIVE
:END:

## Mission

This note is the `P0 / Mapping Gate` inventory for the active Wendao
`core` / `runtime` / plugin-package migration.

Primary references:

- `[[docs/rfcs/2026-03-27-wendao-core-runtime-plugin-migration-rfc.md]]`
- `[[docs/rfcs/2026-03-27-wendao-arrow-plugin-flight-rfc.md]]`
- `[[.data/blueprints/wendao_arrow_plugin_core_runtime_migration.md]]`
- `[[06_roadmap/405_large_rust_modularization]]`
- `[[06_roadmap/410_p1_generic_plugin_contract_staging]]`

This document records the current Julia-specific host surfaces that must be
generalized or relocated before crate extraction and independent plugin
publication can proceed safely.

## Gate Intent

`Gate P0` requires:

1. a complete inventory of the current Julia-specific host surfaces
2. an ownership decision for each surface
3. a target namespace for each migrated responsibility
4. an explicit modularization expectation for every touched medium or complex
   seam

The migration must not begin crate extraction until this map is stable.

## Classification Rules

This inventory uses the following target owners:

1. `core`
   - stable capability, artifact, schema, and transport contracts
   - no process lifecycle, no language-specific runtime settings
2. `runtime`
   - process launch, transport negotiation, routing, health, fallback, and
     UI-facing host assembly
3. `xiuxian-wendao-julia`
   - Julia-specific capability declarations, launch details, deployment
     artifacts, and plugin-owned transport defaults

This inventory also uses the following structural rule:

1. medium or complex migration slices must end in a feature folder with
   responsibility-oriented leaf files
2. `mod.rs` remains interface-only
3. compatibility shims may preserve public exports, but implementation must
   move behind the new namespace

## Current Surface Inventory

| Current surface | Current path | Current problem | Target owner | Target namespace | Planned phase |
| :--- | :--- | :--- | :--- | :--- | :--- |
| Julia-specific runtime config records such as `LinkGraphJuliaRerankRuntimeConfig`, `LinkGraphJuliaAnalyzerServiceDescriptor`, `LinkGraphJuliaAnalyzerLaunchManifest`, and `LinkGraphJuliaDeploymentArtifact` | `src/link_graph/runtime_config/models.rs` | Stable runtime config is encoded as Julia-only types inside the host | `runtime` and `xiuxian-wendao-julia` | `runtime/runtime_config/providers/`, `runtime/artifacts/`, `xiuxian-wendao-julia/capabilities/`, `xiuxian-wendao-julia/artifacts/`, `xiuxian-wendao-julia/launch/` | `P1`, `P3`, `P4` |
| `link_graph.retrieval.julia_rerank` host config path | `src/link_graph/runtime_config/models.rs`, `src/link_graph/runtime_config/resolve/policy.rs` | Provider identity is hardcoded into the config shape, which blocks generic capability routing | `runtime` | `runtime/runtime_config/capabilities/`, `runtime/negotiation/` | `P1`, `P3` |
| Julia-specific environment variables and launcher defaults such as `XIUXIAN_WENDAO_LINK_GRAPH_JULIA_RERANK_*` and `.data/WendaoAnalyzer/scripts/run_analyzer_service.sh` | `src/link_graph/runtime_config/constants.rs` | Host runtime defaults are language-scoped rather than provider-scoped | `runtime` with Julia-owned defaults in plugin package | `runtime/runtime_config/providers/`, `runtime/launch/`, `xiuxian-wendao-julia/manifest/`, `xiuxian-wendao-julia/launch/` | `P1`, `P3`, `P4` |
| Legacy Studio compatibility artifact JSON shape and route-local JSON wrapper | `src/gateway/studio/router/handlers/capabilities/deployment.rs` | The compat route still preserves Julia-compatible field grouping instead of exposing only the generic plugin-artifact payload | `runtime` | `runtime/gateway/studio/router/handlers/plugin_artifacts/` | `P1`, `P5` |
| Julia-only Studio route `get_julia_deployment_artifact` and endpoint wiring | `src/gateway/studio/router/handlers/capabilities/deployment.rs`, `src/gateway/studio/router/routes.rs` | Gateway exposes a language-specific artifact path rather than a plugin artifact surface | `runtime` | `runtime/gateway/studio/router/handlers/plugin_artifacts/`, `runtime/gateway/studio/router/routes/` | `P3`, `P5` |
| Compatibility deployment-artifact JSON-RPC and native tool surfaces such as `wendao.compat_deployment_artifact` | `src/zhenfa_router/native/deployment.rs`, `src/zhenfa_router/rpc.rs`, `src/zhenfa_router/http.rs` | RPC contract still exposes a compat-specific artifact export instead of routing everything through the generic plugin-artifact selector surface | `runtime` | `runtime/zhenfa/artifacts/`, `runtime/zhenfa/rpc/`, `runtime/zhenfa/http/` | `P3`, `P5` |
| Builtin Julia registration in host bootstrap | `src/analyzers/service/bootstrap.rs` | The host owns Julia plugin assembly directly instead of loading a package-defined provider | `runtime` and `xiuxian-wendao-julia` | `runtime/registry/`, `runtime/discovery/`, `xiuxian-wendao-julia/entry/` | `P3`, `P4` |
| Former sibling-source inclusion hack for Julia plugin code | `src/analyzers/languages/mod.rs` | Julia previously entered the host through `#[path]`; the current tree now uses a normal crate dependency, so this row remains only as a retirement checkpoint and as a reminder that Modelica still needs the same treatment | resolved for Julia, still relevant as pattern guidance for remaining plugins | `xiuxian-wendao-julia/plugin/` with package dependency registration instead of source inclusion | `P4` |
| Julia-specific rerank planning and transport helpers | `src/link_graph/index/search/plan/payload/quantum.rs` | Capability execution path is hardcoded to Julia rather than routed through a generic provider binding | `runtime` with Julia-specific transport details in plugin package | `runtime/capabilities/rerank/`, `runtime/transport/`, `runtime/negotiation/`, `xiuxian-wendao-julia/transport/`, `xiuxian-wendao-julia/capabilities/rerank/` | `P1`, `P3`, `P4` |
| Julia-specific request-batch builder names in ignition helpers | `src/link_graph/index/search/quantum_fusion/openai_ignition.rs`, `src/link_graph/index/search/quantum_fusion/vector_ignition.rs` | Shared preparation logic is named after one provider even though the long-term host contract is capability-oriented | `runtime` | `runtime/capabilities/rerank/request/` | `P1`, `P3` |
| Link-graph public re-exports of Julia-specific types | `src/link_graph/mod.rs`, `src/link_graph/runtime_config.rs` | The link-graph domain surface leaks one plugin provider as core vocabulary | `core` compatibility shim plus `runtime` implementation | `core/capabilities/`, `runtime/capabilities/`, `runtime/artifacts/` | `P1`, `P2`, `P5` |
| Julia-specific test fixtures and planned integration tests | `tests/integration/planned_search_julia_rerank*.rs`, `tests/integration/support/wendaoarrow_official_examples.rs`, `src/gateway/studio/router/tests/config.rs` | The test topology mirrors the current host leak and must migrate alongside the runtime and plugin seams | split across `runtime` and `xiuxian-wendao-julia` | `runtime/tests/capabilities/rerank/`, `runtime/tests/artifacts/`, `xiuxian-wendao-julia/tests/launch/`, `xiuxian-wendao-julia/tests/artifacts/` | `P3`, `P4`, `P5` |

## Immediate Ownership Decisions

The current inventory resolves the previously ambiguous boundaries as follows:

1. `core` keeps only generic capability, artifact, and schema contracts.
2. `runtime` owns every host behavior that launches, negotiates with, routes
   to, or renders plugin providers.
3. `xiuxian-wendao-julia` owns Julia-specific launch metadata, deployment
   artifact payload shape, transport defaults, and capability declarations.
4. temporary Julia-named public exports may remain only as compatibility
   shims while the generic runtime surface becomes authoritative.

## Structural Namespace Targets

The first stable namespace targets for migration are:

```text
xiuxian-wendao-core
  capabilities/
  artifacts/
  transport/
  schemas/

xiuxian-wendao-runtime
  capabilities/
    rerank/
  artifacts/
    resolve/
    render/
  runtime_config/
    capabilities/
    providers/
  transport/
  negotiation/
  registry/
  discovery/
  launch/
  health/
  telemetry/
  gateway/
    studio/
      router/
        handlers/
          plugin_artifacts/
      types/
        artifacts/

xiuxian-wendao-julia
  plugin/
  capabilities/
    rerank/
  artifacts/
  launch/
  manifest/
  transport/
  tests/
```

Every touched medium or complex slice must land in one of these
responsibility-oriented folders rather than in a new flat host file.

## Compatibility Rules

During migration, the following compatibility rules apply:

1. legacy Julia-named public exports may remain temporarily if they delegate to
   the new generic owner
2. new implementation logic must not be added behind the legacy Julia-named
   facade
3. new plugin providers must use the generic capability and artifact surfaces
   rather than copying the Julia naming pattern

Current live status note:

- Julia-owned launch/deployment DTO meaning and selector ownership now live in
  `packages/rust/crates/xiuxian-wendao-julia/src/compatibility/link_graph/`,
  so the host `launch.rs` and `artifact.rs` files now behave as compatibility
  re-export seams instead of owning those records directly
- the same Julia compatibility folder now also owns
  `LinkGraphJuliaAnalyzerServiceDescriptor` and the Julia analyzer-launch
  CLI-arg mapping, along with the default Julia analyzer launcher path, so
  the remaining host ownership had been narrowed to
  `LinkGraphJuliaRerankRuntimeConfig` plus package-path/default ownership
- the Julia package-path/default seam now lives in
  `packages/rust/crates/xiuxian-wendao-julia/src/compatibility/link_graph/paths.rs`,
  which owns the default analyzer package dir, launcher path, and example
  config path; the touched host runtime/tests and integration fixtures now
  consume those Julia-owned constants instead of embedding raw
  `.data/WendaoAnalyzer/...` literals
- the Julia rerank runtime-record seam now also lives in
  `packages/rust/crates/xiuxian-wendao-julia/src/compatibility/link_graph/runtime.rs`,
  which owns `LinkGraphJuliaRerankRuntimeConfig` and its provider-binding /
  launch / artifact normalization helpers; the host `runtime.rs` and
  `conversions.rs` files now serve only as compatibility wrappers
- the Zhenfa-side test-only Julia deployment aliases and shim functions are
  now explicitly grouped under `src/zhenfa_router/native/compatibility/`,
  which keeps the compat-first implementation path and the legacy Julia test
  seam physically separated
- the first `M5` outward-surface cutover is now live too: Studio routing and
  OpenAPI inventory expose the canonical generic plugin-artifact path
  `/api/ui/plugins/{plugin_id}/artifacts/{artifact_id}`, while the compat
  deployment-artifact route remains as a wrapper over that same generic
  artifact resolution/render path
- the same `M5` cutover has now reached Zhenfa as well: the canonical outward
  tool/RPC surface is `wendao.plugin_artifact`, while the narrowed
  compat-specific surface is now `wendao.compat_deployment_artifact` over the
  same selector-based export path
- the Studio UI payload seam has now tightened as well: `UiPluginArtifact`
  is the primary Studio artifact payload, while
  the remaining legacy compat JSON shape is now built from the generic
  UI payload instead of reading the core plugin-artifact record directly
- the Studio schema-export seam now follows the same pattern:
  `studio_type_collection()` registers the generic artifact types and the
  `export_types` binary compiles against that generic-only artifact schema
  path without promoting `UiJuliaDeploymentArtifact` into the canonical
  TypeScript-facing collection
- the remaining Julia UI DTO exposure has now narrowed one layer further:
  `UiJuliaDeploymentArtifact` no longer rides through the compatibility
  namespace root and now survives only as route-local compat JSON adaptation
  inside `src/gateway/studio/router/handlers/capabilities/deployment.rs`
- the remaining router-level consumers have now narrowed as well:
  `UiJuliaDeploymentArtifact` is no longer deserialized directly in router
  tests, so the legacy DTO shape now stays covered in the compatibility leaf
  while higher-level Studio tests assert outward JSON payloads generically
- the compat handler seam has now narrowed one layer further too:
  the route layer no longer imports `UiJuliaDeploymentArtifact` directly and
  instead delegates legacy JSON shaping through a route-local wrapper over
  `UiPluginArtifact`
- the test-only Studio Julia route/query shim has now been retired too:
  `JuliaDeploymentArtifactQuery` and `get_julia_deployment_artifact` are gone,
  and legacy regression coverage now targets the compat handler directly
- the OpenAPI Julia route-path aliases are now retired from code:
  `API_UI_JULIA_DEPLOYMENT_ARTIFACT_*` are gone from the live tree, and the
  route inventory now validates only the canonical plugin-artifact path plus
  the compat deployment-artifact path
- the Zhenfa Julia tool entry point has now entered the same retirement track:
  `wendao_julia_deployment_artifact` is deprecated, while the high-level
  default stays on `WendaoPluginArtifactTool` and
  `WendaoCompatDeploymentArtifactTool`
- the remaining Julia RPC/helper family in Zhenfa has now been tightened as a
  whole: the test-only RPC shim is gone, and the surviving Julia-named helper
  aliases in `native/compatibility/julia_deployment.rs` are now explicitly
  deprecated compatibility seams
- the old Zhenfa native compatibility helper folder is now retired entirely:
  `native/compatibility/` is gone, native deployment tests now live directly
  under `deployment.rs`, and the former test-only Julia helper/type aliases no
  longer exist in the tree
- the crate-root and `runtime_config` top-level Julia-named DTO/helper exports
  are now retired too: those names live only under the explicit compatibility
  namespaces instead of leaking through flat public re-export blocks
- the former crate-root Julia compatibility shim is now retired from code too:
  `src/compatibility/julia.rs` has been deleted, so
  `src/compatibility/link_graph.rs` is now the only remaining crate-root
  compatibility surface in the live tree

## Current Compatibility Ledger

The current live tree now has a narrower set of Julia-named outward surfaces.
These should be treated as an explicit compatibility ledger rather than as
default host vocabulary.

### Legacy Julia names that still remain intentionally

| Surface | Current path | Why it still exists | Migration rule |
| :--- | :--- | :--- | :--- |
| Legacy Studio compatibility artifact JSON shape | `src/gateway/studio/router/handlers/capabilities/deployment.rs` | Existing Studio-facing JSON payloads still preserve Julia-compatible field grouping on the compat route | Keep as a route-local JSON wrapper over `UiPluginArtifact`; do not reintroduce a dedicated compatibility type leaf or canonical Studio type export |

### Julia names that are now compatibility-seam only

These surfaces have already been pushed behind narrower ownership seams and
should not reappear on higher-level host surfaces:

1. `UI_JULIA_DEPLOYMENT_ARTIFACT` route-contract inventory alias
2. `get_julia_deployment_artifact` and `JuliaDeploymentArtifactQuery` as
   higher-level capability-module re-exports
3. Julia deployment-artifact helpers as `link_graph` middle-layer re-exports
4. raw `JULIA_*` ids as high-level host re-exports
5. Julia deployment tool type as the default `zhenfa_router` export
6. `UiJuliaDeploymentArtifact` as a root-level `types::compatibility::*`
   re-export
7. router-level direct deserialization of `UiJuliaDeploymentArtifact`
8. route-layer direct imports of `UiJuliaDeploymentArtifact`
9. Julia-named Zhenfa native deployment Rust tool/helper symbols
10. OpenAPI Julia path alias constants
11. Legacy Julia Zhenfa outward tool name
12. Flat crate-root and `runtime_config` root Julia re-export blocks
13. Julia-named Studio compatibility Rust DTO symbols

## Compatibility Namespace Map

The current compatibility seams are now physically grouped as follows:

```text
src/compatibility/
  link_graph.rs

src/gateway/studio/router/handlers/capabilities/
  deployment.rs
```

The crate-root compatibility shim work should remain under
`src/compatibility/`. The remaining Studio compat JSON adaptation is now
route-local in the deployment handler and should not be promoted back into a
dedicated type compatibility folder.

The crate root now exposes an explicit `src/compatibility/` namespace. Inside
that namespace, `src/compatibility/link_graph.rs` is the canonical compat-first
path and the only remaining crate-root compatibility module.

The crate-root flat Julia re-export block is gone, the `runtime_config`
compatibility sub-namespace has been retired, and the former
`src/compatibility/julia.rs` shim has now been deleted as well. The remaining
host-side legacy regression no longer imports any Julia-named crate-root helper
path from `src/link_graph/runtime_config/tests.rs`.

Downstream migration guidance now becomes:

1. prefer `crate::compatibility::link_graph::*` for compat-first runtime-config
   DTO imports and deployment helpers
2. treat Julia-named crate-root helper imports as retired from host code
3. move remaining legacy Julia compatibility usage into package-owned or
   leaf-level compatibility seams instead of restoring a crate-root Julia shim

### Next removal / generalization candidates

The next outward surfaces most likely to move after `P1` are:

1. Studio route-local compat JSON adapter retirement once generic consumers
   can accept the primary plugin-artifact payload directly
2. any remaining `UiJulia*` historical wording in tests or notes once generic
   plugin artifact UI payloads become the canonical external contract

## Exit Criteria for `Gate P0`

`Gate P0` is complete when:

1. this inventory remains accurate for the live tree
2. every listed surface has a resolved owner
3. every listed medium or complex seam has a target feature-folder namespace
4. `P1` implementation work can proceed without reopening ownership debates

:RELATIONS:
:LINKS: [[index]], [[06_roadmap/404_repo_intelligence_for_sciml_and_msl]], [[06_roadmap/405_large_rust_modularization]], [[docs/rfcs/2026-03-27-wendao-core-runtime-plugin-migration-rfc.md]], [[.data/blueprints/wendao_arrow_plugin_core_runtime_migration.md]]
:END:

---

:FOOTER:
:STANDARDS: v2.0
:LAST_SYNC: 2026-03-28
:END:
