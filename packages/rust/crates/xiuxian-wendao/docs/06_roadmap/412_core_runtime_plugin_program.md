# Wendao Core Runtime Plugin Program

:PROPERTIES:
:ID: wendao-core-runtime-plugin-program
:PARENT: [[index]]
:TAGS: roadmap, migration, plugins, core, runtime, program
:STATUS: ACTIVE
:END:

## Purpose

This note is the program-level execution entrypoint for the Wendao
core/runtime/plugin migration.

It exists to stop fragmented implementation drift and to turn the active RFC,
blueprint, inventory, and `P1` notes into one coordinated rollout plan.

Primary references:

- `[[docs/rfcs/2026-03-27-wendao-core-runtime-plugin-migration-rfc.md]]`
- `[[docs/rfcs/2026-03-27-wendao-arrow-plugin-flight-rfc.md]]`
- `[[.data/blueprints/wendao_arrow_plugin_core_runtime_migration.md]]`
- `[[06_roadmap/409_core_runtime_plugin_surface_inventory]]`
- `[[06_roadmap/410_p1_generic_plugin_contract_staging]]`
- `[[06_roadmap/411_p1_first_code_slice_plan]]`

## Program Position

The active tree is now in early `M5`.

Completed baseline:

1. generic plugin-runtime vocabulary exists in the current tree
2. compatibility seams are explicit feature folders
3. crate-root compatibility export maps now exist
4. public migration paths are documented in rustdoc, README, and roadmap notes
5. the first physical `xiuxian-wendao-core` crate cut now exists in the
   workspace

Incomplete baseline:

1. `xiuxian-wendao-core` is not yet wired into every main-crate consumer
2. `xiuxian-wendao-runtime` is not yet wired into every live host assembly and
   resolver seam
3. Julia ownership externalization is in progress but not yet complete
4. generic plugin-artifact outward surfaces are now partially canonical, but
   they are not yet the only canonical endpoint family across every outward
   consumer and payload/tool alias

## Macro Phases

### M1: Contract and Compatibility Stabilization

Scope:

1. complete contract normalization
2. finish narrowing host-side compatibility seams
3. freeze the `core` extraction candidate surface

Exit criteria:

1. no new language-specific host types land outside explicit compatibility
   namespaces
2. `core` extraction package list is complete
3. `runtime` extraction package list is complete

### M2: Core Extraction

Scope:

1. create `xiuxian-wendao-core`
2. move stable contracts there
3. preserve compatibility re-exports temporarily

Exit criteria:

1. `core` builds independently
2. semver-governed contract surface is physically separated

### M3: Runtime Extraction

Scope:

1. create `xiuxian-wendao-runtime`
2. move launch, negotiation, routing, health, and fallback ownership
3. connect binaries and host assembly through runtime

Exit criteria:

1. runtime behavior no longer depends on the monolithic crate boundary
2. `core` remains free of lifecycle ownership

### M4: Julia Ownership Externalization

Scope:

1. create or finalize the independently publishable Julia package path
2. move Julia-owned launch and artifact responsibilities there
3. remove in-tree source inclusion hacks

Exit criteria:

1. Julia package owns Julia-specific runtime details
2. host integration uses plugin contracts, not host-owned Julia DTOs

### M5: Generic Artifact Cutover

Scope:

1. make generic plugin-artifact endpoints canonical
2. demote Julia outward surfaces to compatibility-only

Exit criteria:

1. Studio, OpenAPI, and Zhenfa prefer generic artifact contracts
2. Julia-named outward endpoints are compatibility shims only

### M6: Additional Plugin Proof

Scope:

1. onboard one more language/plugin path
2. verify that the new architecture is additive

Exit criteria:

1. one additional plugin path lands without core expansion

## Program Deliverables

The following deliverables must now be kept current:

1. `core` extraction package list
2. `runtime` extraction package list
3. Julia externalization package list
4. compatibility retirement ledger

## Immediate Next Program Move

The next overall move should be:

1. continue `M5` beyond the first canonical Studio/OpenAPI artifact endpoint
2. propagate the same generic plugin-artifact surface through the remaining
   outward payload and compatibility seams
3. keep Julia-named outward surfaces compatibility-only
4. retire outward compatibility seams in the order defined by
   `[[06_roadmap/416_compatibility_retirement_ledger]]`

Current status:

1. the `M2` core extraction package list now exists in
   `[[06_roadmap/413_m2_core_extraction_package_list]]`
2. the first physical `xiuxian-wendao-core` crate cut now exists in the
   workspace and the first plugin-runtime contract slice in
   `xiuxian-wendao` now re-exports from it
3. the `M3` runtime extraction package list now exists in
   `[[06_roadmap/414_m3_runtime_extraction_package_list]]`
4. the Julia externalization package list now exists in
   `[[06_roadmap/415_m4_julia_externalization_package_list]]`
5. the compatibility retirement ledger now exists in
   `[[06_roadmap/416_compatibility_retirement_ledger]]`
6. the program artifact set is now complete for `M2` through `M5` planning
7. the next overall implementation move is to expand consumer cutover beyond
   the `plugin_runtime` barrel and into the remaining `M2` contract surface
8. that expansion has now started in `runtime_config`, where the pure
   contract-side Julia rerank selector, binding, launch, artifact, and
   transport imports are being sourced from `xiuxian-wendao-core`
9. the same `M2` slice has now reached outward consumers too:
   `runtime_config.rs` and the Studio UI type compatibility/config modules
   now import stable launch, artifact, and binding records from
   `xiuxian-wendao-core`
10. the next `M2` cutover ring has now reached helper-consumer modules:
    artifact resolution, transport-client assembly, and quantum rerank flow
    modules now consume stable contract records from `xiuxian-wendao-core`
11. the remaining selector/enum-only consumers under `plugin_runtime/` are
    also being cleaned up so that even focused helper/test seams stop reading
    stable contract records through the monolithic crate layer
12. `M3` physical extraction has now started too: the first
    `xiuxian-wendao-runtime` crate cut exists in the workspace and owns the
    transport-client construction slice, while `xiuxian-wendao` keeps a
    temporary re-export seam for compatibility
13. `M3` has now expanded to a second runtime-owned helper slice too:
    generic artifact render behavior lives in `xiuxian-wendao-runtime`, while
    the monolithic crate keeps only the runtime-config-backed resolver seam
14. `M3` now also owns the generic artifact resolve helper in
    `xiuxian-wendao-runtime`; the monolithic crate keeps only the
    runtime-state-backed resolver callback for the current Julia compatibility
    path
15. `M3` now owns the generic runtime-config settings helper seam too:
    `xiuxian-wendao-runtime/src/settings/` holds the override, TOML-merge,
    parse, access, and directory helpers, while
    `src/link_graph/runtime_config/settings/mod.rs` in `xiuxian-wendao`
    retains only the Wendao-embedded config wrapper and module-shaped
    re-export surface expected by the local resolve tree
16. `M3` now owns the first live runtime-config resolution slice as well:
    cache, related, coactivation, and index-scope records/constants/resolvers
    now live in `xiuxian-wendao-runtime/src/runtime_config/`, while
    `xiuxian-wendao` retains only the settings-backed wrapper layer that keeps
    the original module paths stable
17. `M3` now owns the agentic runtime subtree too:
    `xiuxian-wendao-runtime/src/runtime_config/models/agentic.rs` and
    `src/runtime_config/resolve/agentic/` hold the record/default/env/apply/
    finalize ownership, while `xiuxian-wendao` keeps only the
    `merged_wendao_settings()` wrapper boundary
18. `M3` now owns the generic retrieval semantic-ignition subtree too:
    `xiuxian-wendao-runtime/src/runtime_config/retrieval/semantic_ignition.rs`
    holds the record/default/env/settings-to-runtime resolver ownership, while
    `xiuxian-wendao` keeps only the existing model/resolver module paths as
    re-export seams
19. `M3` now owns the generic retrieval tuning/base slice as well:
    `xiuxian-wendao-runtime/src/runtime_config/retrieval/base.rs` resolves
    candidate multiplier, max sources, graph sufficiency thresholds, graph
    rows per source, and semantic-ignition integration, while
    `xiuxian-wendao` keeps only the `mode + julia_rerank` assembly wrapper in
    `resolve/policy/retrieval/base.rs`
20. `M2` now also owns the first analyzer contract extraction slice:
    `xiuxian-wendao-core/src/repo_intelligence/` holds repo-intelligence
    config records, plugin traits/context/output, record types, registry,
    `RepoIntelligenceError`, `ProjectionPageKind`, and the Julia Arrow
    analyzer transport column/schema contracts
21. that analyzer slice is now wired into both the monolithic host and the
    Julia package: the main-crate analyzer contract modules now re-export from
    `xiuxian-wendao-core`, and `xiuxian-wendao-julia` now imports those
    contracts from `core`
22. `M4` now owns the Julia link-graph launch/artifact compatibility slice:
    `xiuxian-wendao-julia/src/compatibility/link_graph/` now holds the Julia
    selector ids/helpers, `LinkGraphJuliaAnalyzerServiceDescriptor`,
    `LinkGraphJuliaAnalyzerLaunchManifest`,
    `LinkGraphJuliaDeploymentArtifact`, the Julia launch-option arg mapping,
    the default Julia analyzer launcher path, and the conversion boundary to
    and from Wendao core plugin contracts
23. the monolithic host now keeps
    `src/link_graph/runtime_config/models/retrieval/julia_rerank/{launch,artifact}.rs`
    only as compatibility re-export seams over that Julia-owned slice, while
    `runtime.rs` delegates Julia analyzer-launch arg encoding into the Julia
    crate
24. the remaining `M4` blockers were, until the previous slice, concentrated
    in still-hosted Julia runtime defaults and package-path semantics,
    especially `LinkGraphJuliaRerankRuntimeConfig` and package-path/default
    ownership
25. `M4` now owns the Julia package-path/default slice too:
    `xiuxian-wendao-julia/src/compatibility/link_graph/paths.rs` is now the
    physical owner of the default analyzer package dir, launcher path, and
    example-config path, while the host runtime/tests and integration fixtures
    consume those Julia-owned constants
26. `M4` now also owns the Julia rerank runtime record itself:
    `xiuxian-wendao-julia/src/compatibility/link_graph/runtime.rs` is now the
    physical owner of `LinkGraphJuliaRerankRuntimeConfig` and its
    provider-binding / launch / artifact normalization methods, while the host
    `runtime.rs` and `conversions.rs` files now behave as compatibility seams
27. as a result, the hard `M4` ownership blockers are now cleared and the
    next overall program move should be `M5` generic artifact cutover plus
    compatibility retirement sequencing
28. `M4` has now crossed the first dependency-rewrite milestone too:
    `xiuxian-wendao-julia` no longer depends on `xiuxian-wendao` directly and
    now builds against `xiuxian-wendao-core` plus `xiuxian-vector`
29. `M4` has now crossed the first host-integration milestone too:
    `src/analyzers/languages/mod.rs` no longer uses sibling-source inclusion
    for Julia and now loads `xiuxian-wendao-julia` through a normal crate
    dependency; the remaining path-inclusion seam is Modelica-specific
30. `M5` has now started with the first canonical generic outward artifact
    cutover: Studio routing and OpenAPI inventory now expose
    `/api/ui/plugins/{plugin_id}/artifacts/{artifact_id}` as the generic
    plugin-artifact endpoint family
31. the compat deployment-artifact route remains live, but the compat
    deployment-artifact handler and route now behave as wrappers over the same
    plugin-artifact resolution/render path instead of owning the primary
    outward implementation logic
32. `M5` has now expanded into Zhenfa too: the router now exposes
    `wendao.plugin_artifact` as the canonical generic tool/RPC surface, while
    `wendao.compat_deployment_artifact` remains as the compat-specific wrapper
33. `M5` has now pushed the Studio UI payload seam further too:
    `UiPluginArtifact` is now the primary Studio artifact payload, while
    `UiJuliaDeploymentArtifact` stays under `types/compatibility/` and is
    built from the generic UI payload rather than directly from the core
34. `M5` has now retired the Julia-named Studio compatibility Rust symbols
    too: the compat route still preserves the legacy Julia-shaped JSON
    payload, but the remaining internal adapter is now compat-first rather
    than Julia-named
35. the canonical Studio schema-export seam now follows the same rule:
    `studio_type_collection()` and the `export_types` binary now register and
    compile only the generic artifact types, so the TypeScript-facing artifact
    schema path no longer needs the Julia DTO as a primary export
36. the remaining Julia UI DTO exposure has now been narrowed further too:
    `UiJuliaDeploymentArtifact` no longer rides through the compatibility
    namespace root and now survives only as route-local compat JSON
    adaptation in the deployment handler
37. the same `M5` cutover has now tightened the remaining Studio compatibility
    consumers too: router-level tests no longer deserialize
    `UiJuliaDeploymentArtifact` directly and instead assert the outward JSON
    payload through generic/value checks, leaving the legacy DTO shape
    coverage inside the compatibility leaf itself
38. the same `M5` cutover has now narrowed the compat handler seam further:
    the route layer no longer imports `UiJuliaDeploymentArtifact` directly and
    instead delegates legacy JSON shaping through a route-local wrapper over
    `UiPluginArtifact`
39. the same `M5` retirement path has now deleted the last test-only Studio
    Julia route/query shim too: `JuliaDeploymentArtifactQuery` and
    `get_julia_deployment_artifact` are gone, and legacy regression coverage
    now targets the compat handler directly
40. the same `M5` retirement path has now completed the OpenAPI Julia path
    alias removal too: `API_UI_JULIA_DEPLOYMENT_ARTIFACT_*` are gone from the
    codebase, and the route inventory now validates only the canonical plugin
    artifact path plus the compat deployment-artifact path
41. the same `M5` retirement path has now completed the Zhenfa Julia outward
    tool-name retirement too: `wendao.julia_deployment_artifact` is gone from
    the live code path, and the remaining outward tool surface is now the
    pair `wendao.plugin_artifact` plus `wendao.compat_deployment_artifact`
42. the same `M5` retirement path has now completed the crate-root and
    `runtime_config` top-level Julia export retirement too: the Julia-named
    DTOs and deployment helpers no longer leak through flat crate-root or
    `runtime_config` root re-exports
43. the same `M5` retirement path has now retired the crate-root
    `src/compatibility/julia.rs` shim itself: `src/compatibility/link_graph.rs`
    is now the only remaining crate-root compatibility surface in the live
    tree
44. the same `M5` retirement path has now retired the last Julia-named Studio
    compatibility leaf path too: the dedicated Studio compatibility type
    module is gone, and the remaining legacy payload adapter is route-local in
    `src/gateway/studio/router/handlers/capabilities/deployment.rs`
45. that same route-local adapter has now narrowed one layer further too: the
    compat route no longer maintains a parallel Rust DTO and instead wraps the
    generic `UiPluginArtifact` into the legacy JSON shape at the serialization
    boundary
46. the next overall program move should therefore stay inside `M5` and push
    the same generic outward artifact surface through the remaining payload,
    DTO, and compatibility-retirement seams

## Governance Rule

Any future implementation note, ExecPlan, or code slice that affects this
program should explicitly state:

1. macro phase
2. gate
3. ownership seam
4. compatibility impact

If it does not, it is not yet ready to be treated as migration-program work.

:RELATIONS:
:LINKS: [[index]], [[06_roadmap/409_core_runtime_plugin_surface_inventory]], [[06_roadmap/410_p1_generic_plugin_contract_staging]], [[06_roadmap/411_p1_first_code_slice_plan]], [[06_roadmap/413_m2_core_extraction_package_list]], [[06_roadmap/414_m3_runtime_extraction_package_list]], [[06_roadmap/415_m4_julia_externalization_package_list]], [[06_roadmap/416_compatibility_retirement_ledger]], [[docs/rfcs/2026-03-27-wendao-core-runtime-plugin-migration-rfc.md]], [[.data/blueprints/wendao_arrow_plugin_core_runtime_migration.md]]
:END:

---
