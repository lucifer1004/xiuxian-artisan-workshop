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

The active tree has now cleared `M6` and is at the handoff to the next
macro-phase target defined in the RFC: `Phase 7: Flight-First Runtime
Negotiation`.

Completed baseline:

1. generic plugin-runtime vocabulary exists in the current tree
2. compatibility seams are explicit feature folders
3. host crate-root compatibility export maps have now been retired after
   serving the migration cutover
4. the remaining Julia-specific compatibility ownership is package-owned in
   `xiuxian-wendao-julia`
5. the first physical `xiuxian-wendao-core` crate cut now exists in the
   workspace
6. the first `M6` additive plugin proof now exists too: Modelica uses normal
   package dependencies instead of host-side source inclusion

Incomplete baseline:

1. `xiuxian-wendao-core` is not yet wired into every main-crate consumer
2. `xiuxian-wendao-runtime` is not yet wired into every live host assembly and
   resolver seam
3. package-owned Julia compatibility names may still need downstream cleanup
   after the host-side retirement cut

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

Authoritative current position:

1. the additive-proof track has now cleared `M6`
2. the current risk is no longer proof coverage; it is transport/runtime
   hardening ambiguity for the next macro phase
3. the next push should therefore target `Phase 7: Flight-First Runtime
   Negotiation` rather than another additive-proof slice

Phase-7 staged push plan:

1. `Stage A: Transport Surface Inventory Bundle`
   identify the live negotiation seams, fallback callers, and outward
   diagnostics surfaces, then document one canonical transport preference
   order
2. `Stage B: Negotiation Policy Bundle`
   harden runtime selection so Flight is preferred where supported while Arrow
   IPC remains the bounded fallback path
3. `Stage C: Observability and Gate Bundle`
   expose negotiated transport plus fallback reason, then run one explicit
   `Phase 7` go/no-go review

Current staged position:

1. `Stage A` is complete
2. `Stage B` is complete
3. `Stage C` is now the active next move

Stage-A inventory summary:

1. the generic contract surface is already stable in `xiuxian-wendao-core`
   via `PluginCapabilityBinding`, `PluginTransportEndpoint`, and
   `PluginTransportKind`
2. the only live runtime-owned transport-construction seam today is
   `xiuxian-wendao-runtime/src/transport/client.rs`, which currently builds
   `ArrowIpcHttp` clients from generic capability bindings
3. the current outward inspection seam is `UiPluginArtifact`, which already
   carries endpoint metadata but does not yet report negotiated transport or
   fallback reason
4. the canonical Phase-7 preference order is now fixed as
   `ArrowFlight -> ArrowIpcHttp -> LocalProcessArrowIpc`
5. the first Stage-B runtime cut had landed in
   `xiuxian-wendao-runtime/src/transport/negotiation.rs`, and the rerank path
   now delegates through that runtime-owned negotiation seam
6. the second Stage-B cut has now landed in
   `xiuxian-wendao-runtime/src/transport/flight.rs`, where the runtime owns a
   real Flight client materialization seam aligned to the LanceDB Arrow
   `57.3` line
7. the runtime now bridges host-side Arrow-58 rerank batches onto that
   LanceDB Arrow-57 Flight line through the existing
   `xiuxian-vector::{engine_batches_to_lance_batches,
   lance_batches_to_engine_batches}` compat seam
8. the rerank path can now negotiate and materialize both the preferred
   `ArrowFlight` path and the bounded `ArrowIpcHttp` fallback, so the next
   governed move is `Stage C: Observability and Gate Bundle`

## M6 Exit Review

Decision: `go`

The `M6` completion conditions are now satisfied:

1. one non-Julia plugin path has landed without new language-specific host
   structs
2. repo-facing, docs-facing, and Studio-facing consumers all now have bounded
   additive proof coverage
3. the RFC, active ExecPlan, program note, outward inventory, and package note
   now agree on the same position and governed next move

Next macro-phase target:

1. `Phase 7: Flight-First Runtime Negotiation`
2. transport preference hardening should now replace additive-proof expansion
   as the active program concern

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
    dependency
30. `M6` has now landed its first additive plugin proof too:
    `xiuxian-wendao-modelica` now depends on
    `xiuxian-wendao-core::repo_intelligence` for production contracts, the
    host loads Modelica through a normal optional crate dependency instead of
    sibling-source inclusion, Modelica keeps `xiuxian-wendao` only as a
    dev-dependency for registry-aware integration-query validation, and the
    host `xiuxian-testing-gate` now carries a real builtin-registry
    Modelica repo-overview/module-search/example-search regression
31. that same `M6` proof is now two host consumers deep instead of one:
    the shared support topology under `tests/integration/support/` no longer
    compiles per-file repo helper copies, the historical
    `#[allow(dead_code)]` suppressions in `tests/support/repo_fixture.rs` and
    `tests/support/repo_intelligence.rs` are gone, and the builtin-registry
    Modelica path now has a second real host regression through
    `repo_symbol_search.rs`
32. `M6` has now reached a third host consumer too: the same external
    Modelica path now proves relation-graph output through
    `tests/integration/repo_relations.rs`, so the additive proof is no longer
    limited to overview/search-only consumers
33. `M6` has now reached projected-page lookup too: the external Modelica path
    now proves config-backed projected-page generation and page lookup through
    `tests/integration/repo_projected_page.rs`, so the additive proof has
    crossed from stage-one analysis/search consumers into stage-two docs
    projection
34. `M6` has now reached projected page-index trees too: the same external
    Modelica path now proves config-backed page-index tree generation and
    lookup through `tests/integration/repo_projected_page_index_tree.rs`, so
    the additive proof now covers parsed stage-two hierarchy output as well
35. `M6` has now reached projected page-index nodes too: the same external
    Modelica path now proves config-backed node lookup through
    `tests/integration/repo_projected_page_index_node.rs`, so the additive
    proof now covers stable subtree addressing inside parsed page hierarchies
36. `M6` has now reached page-centric navigation bundles too: the same
    external Modelica path now proves config-backed projected page navigation
    through `tests/integration/repo_projected_page_navigation.rs`, so the
    additive proof now covers assembled stage-two navigation around a real
    external plugin page
37. `M6` has now reached stage-two family context too: the same external
    Modelica path now proves config-backed projected page family context
    through `tests/integration/repo_projected_page_family_context.rs`, so the
    additive proof now covers grouped related-page families around a real
    external plugin page
38. `M6` has now reached singular family-cluster lookup too: the same
    external Modelica path now proves config-backed projected page family
    cluster lookup through `tests/integration/repo_projected_page_family_cluster.rs`,
    so the additive proof now covers direct family selection around a real
    external plugin page
39. `M6` has now reached search-driven family expansion too: the same
    external Modelica path now proves config-backed projected page family
    search through `tests/integration/repo_projected_page_family_search.rs`,
    so the additive proof now covers stable query-to-family expansion around a
    real external plugin page
40. `M6` has now reached search-driven navigation expansion too: the same
    external Modelica path now proves config-backed projected page navigation
    search through `tests/integration/repo_projected_page_navigation_search.rs`,
    so the additive proof now covers stable query-to-navigation bundle
    expansion around a real external plugin page
41. `M5` has now started with the first canonical generic outward artifact
    cutover: Studio routing and OpenAPI inventory now expose
    `/api/ui/plugins/{plugin_id}/artifacts/{artifact_id}` as the generic
    plugin-artifact endpoint family
42. the former Studio compat deployment-artifact route was initially narrowed
    into a wrapper over the generic plugin-artifact resolution/render path
    instead of owning primary outward implementation logic
43. `M5` has now expanded into Zhenfa too: the router now exposes
    `wendao.plugin_artifact` as the canonical generic selector-based
    tool/RPC surface
44. `M5` has now pushed the Studio UI payload seam further too:
    `UiPluginArtifact` is now the primary Studio artifact payload, while
    `UiJuliaDeploymentArtifact` stays under `types/compatibility/` and is
    built from the generic UI payload rather than directly from the core
45. `M5` has now retired the Julia-named Studio compatibility Rust symbols
    too: the compat route still preserves the legacy Julia-shaped JSON
    payload, but the remaining internal adapter is now compat-first rather
    than Julia-named
46. the canonical Studio schema-export seam now follows the same rule:
    `studio_type_collection()` and the `export_types` binary now register and
    compile only the generic artifact types, so the TypeScript-facing artifact
    schema path no longer needs the Julia DTO as a primary export
47. the remaining Julia UI DTO exposure has now been narrowed further too:
    `UiJuliaDeploymentArtifact` no longer rides through the compatibility
    namespace root and now survives only as route-local compat JSON
    adaptation in the deployment handler
48. the same `M5` cutover has now tightened the remaining Studio compatibility
    consumers too: router-level tests no longer deserialize
    `UiJuliaDeploymentArtifact` directly and instead assert the outward JSON
    payload through generic/value checks, leaving the legacy DTO shape
    coverage inside the compatibility leaf itself
49. the same `M5` cutover has now narrowed the compat handler seam further:
    the route layer no longer imports `UiJuliaDeploymentArtifact` directly and
    instead delegates legacy JSON shaping through a route-local wrapper over
    `UiPluginArtifact`
50. the same `M5` retirement path has now deleted the last test-only Studio
    Julia route/query shim too: `JuliaDeploymentArtifactQuery` and
    `get_julia_deployment_artifact` are gone, and legacy regression coverage
    now targets the compat handler directly
51. the same `M5` retirement path has now completed the OpenAPI Julia path
    alias removal too: `API_UI_JULIA_DEPLOYMENT_ARTIFACT_*` are gone from the
    codebase, and the route inventory now validates only the canonical plugin
    artifact path
52. the same `M5` retirement path has now completed the Zhenfa outward
    artifact retirement too: `wendao.julia_deployment_artifact` and
    `wendao.compat_deployment_artifact` are both gone from the live code
    path, so `wendao.plugin_artifact` is now the only Zhenfa artifact
    tool/RPC surface
53. the same `M5` retirement path has now completed the crate-root and
    `runtime_config` top-level Julia export retirement too: the Julia-named
    DTOs and deployment helpers no longer leak through flat crate-root or
    `runtime_config` root re-exports
54. the same `M5` retirement path had first retired the crate-root
    `src/compatibility/julia.rs` shim itself, temporarily narrowing the host
    compatibility surface down to `src/compatibility/link_graph.rs` before
    the final exit-review cut removed that last namespace as well
55. the same `M5` retirement path has now retired the last Julia-named Studio
    compatibility leaf path too: the dedicated Studio compatibility type
    module is gone, and the remaining legacy payload adapter is route-local in
    `src/gateway/studio/router/handlers/capabilities/deployment.rs`
56. that same route-local adapter has now narrowed one layer further too: the
    compat route no longer maintains a parallel Rust DTO and instead wraps the
    generic `UiPluginArtifact` into the legacy JSON shape at the serialization
    boundary
57. that same `M5` retirement path has now completed the Studio/OpenAPI UI
    artifact cutover too: the former `/api/ui/julia-deployment-artifact`
    compat route, query type, handler export, and OpenAPI inventory constants
    are gone from the live tree, so
    `/api/ui/plugins/{plugin_id}/artifacts/{artifact_id}` is now the only
    Studio UI artifact endpoint
58. the next overall program move no longer needs to chase outward artifact
    route/tool retirement; that work is complete on Studio/OpenAPI/Zhenfa
59. the remaining `M5` work is now limited to exit review, consumer cleanup,
    and package-owned Julia compatibility import cleanup
60. that same `M5` exit-review cut first retired the flat crate-root and
    `src/link_graph/mod.rs` compat-first re-export blocks
61. the final host crate-root compatibility namespace is now retired too:
    `src/compatibility/link_graph.rs`, `src/compatibility/mod.rs`, and the
    `pub mod compatibility;` mount in `src/lib.rs` are all gone, and the
    touched internal consumers now import Julia compatibility records from
    `xiuxian-wendao-julia::compatibility::link_graph::*`
62. the next phase transition has therefore already happened:
    `M6` additive plugin proof is now live in the Modelica path, not pending
    behind another host compatibility cycle
63. that same `M6` additive slice now reaches a docs-facing search consumer
    too: `tests/integration/docs_navigation_search.rs` proves
    config-backed `docs_navigation_search_from_config(...)` over the external
    Modelica path, so the additive proof now covers docs-facing
    query-to-navigation bundle expansion as well
64. that same `M6` additive slice now reaches the docs-facing family-search
    peer too: `tests/integration/docs_family_search.rs` proves
    config-backed `docs_family_search_from_config(...)` over the external
    Modelica path, so the additive proof now covers docs-facing
    query-to-family expansion as well
65. that same `M6` additive slice now reaches the docs-facing family-context
    peer too: `tests/integration/docs_family_context.rs` proves
    config-backed `docs_family_context_from_config(...)` over the external
    Modelica path, so the additive proof now covers docs-facing grouped
    family context around a stable external-plugin page as well
66. that same `M6` additive slice now reaches the docs-facing navigation
    lookup peer too: `tests/integration/docs_navigation.rs` proves
    config-backed `docs_navigation_from_config(...)` over the external
    Modelica path, so the additive proof now covers docs-facing deterministic
    navigation lookup with stable node context and family clustering as well
67. that same `M6` additive slice now reaches the docs-facing family-cluster
    lookup peer too: `tests/integration/docs_family_cluster.rs` proves
    config-backed `docs_family_cluster_from_config(...)` over the external
    Modelica path, so the additive proof now covers docs-facing deterministic
    family selection around a stable external-plugin reference page as well
68. that same `M6` additive slice now reaches the docs-facing page lookup
    peer too: `tests/integration/docs_page.rs` proves
    config-backed `docs_page_from_config(...)` over the external Modelica
    path, so the additive proof now covers docs-facing deterministic single
    page lookup over a stable external-plugin symbol page as well
69. that same `M6` additive slice now reaches the docs-facing page-index tree
    lookup peer too: `tests/integration/docs_page_index_tree.rs` proves
    config-backed `docs_page_index_tree_from_config(...)` over the external
    Modelica path, so the additive proof now covers docs-facing deterministic
    parsed page hierarchy lookup over a stable external-plugin symbol page as
    well
70. that same `M6` additive slice now reaches the docs-facing page-index node
    lookup peer too: `tests/integration/docs_page_index_node.rs` proves
    config-backed `docs_page_index_node_from_config(...)` over the external
    Modelica path, so the additive proof now covers docs-facing deterministic
    parsed page section lookup over a stable external-plugin symbol page as
    well
71. that same `M6` additive slice now reaches the docs-facing page-index tree
    search peer too: `tests/integration/docs_page_index_tree_search.rs`
    proves config-backed `docs_page_index_tree_search_from_config(...)` over
    the external Modelica path, so the additive proof now covers docs-facing
    deterministic parsed page hierarchy search over a stable external-plugin
    reference query as well
72. that same `M6` additive slice now reaches the docs-facing page-index
    trees peer too: `tests/integration/docs_page_index_trees.rs` proves
    config-backed `docs_page_index_trees_from_config(...)` over the external
    Modelica path, so the additive proof now covers docs-facing deterministic
    parsed page hierarchy listing over a stable external-plugin repository as
    well
73. that same `M6` additive slice now reaches the docs-facing page-index
    documents peer too: `tests/integration/docs_page_index_documents.rs`
    proves config-backed `docs_page_index_documents_from_config(...)` over
    the external Modelica path, so the additive proof now covers docs-facing
    parsed page-index-ready document generation over a stable external-plugin
    repository as well
74. that same `M6` additive slice now reaches the docs-facing markdown
    documents peer too: `tests/integration/docs_markdown_documents.rs`
    proves config-backed `docs_markdown_documents_from_config(...)` over the
    external Modelica path, so the additive proof now covers docs-facing
    projected markdown document generation over a stable external-plugin
    repository as well
75. that same `M6` additive slice now reaches the docs-facing search peer
    too: `tests/integration/docs_search.rs` proves config-backed
    `docs_search_from_config(...)` over the external Modelica path, so the
    additive proof now covers docs-facing projected page search over a stable
    external-plugin repository as well
76. that same `M6` additive slice now reaches the docs-facing retrieval peer
    too: `tests/integration/docs_retrieval.rs` proves config-backed
    `docs_retrieval_from_config(...)` over the external Modelica path, so the
    additive proof now covers docs-facing mixed projected retrieval over a
    stable external-plugin repository as well
77. that same `M6` additive slice now reaches the docs-facing retrieval-
    context peer too: `tests/integration/docs_retrieval_context.rs` proves
    config-backed `docs_retrieval_context_from_config(...)` over the external
    Modelica path, so the additive proof now covers docs-facing local
    projected retrieval context over a stable external-plugin repository as
    well
78. that same `M6` additive slice now reaches the docs-facing retrieval-hit
    peer too: `tests/integration/docs_retrieval_hit.rs` proves config-backed
    `docs_retrieval_hit_from_config(...)` over the external Modelica path, so
    the additive proof now covers docs-facing deterministic projected
    retrieval-hit reopening over a stable external-plugin repository as well
79. that same `M6` additive slice now reaches the docs-facing projected-gap
    report peer too: `tests/integration/docs_projected_gap_report.rs` proves
    config-backed `docs_projected_gap_report_from_config(...)` over the
    external Modelica path, so the additive proof now covers docs-facing
    projected gap reporting over a stable external-plugin repository as well
80. that same `M6` additive slice now reaches the docs-facing planner-queue
    peer too: `tests/integration/docs_planner_queue.rs` proves config-backed
    `docs_planner_queue_from_config(...)` over the external Modelica path, so
    the additive proof now covers docs-facing deterministic planner queue
    shaping over a stable external-plugin repository as well
81. that same `M6` additive slice now reaches the docs-facing planner-workset
    peer too: `tests/integration/docs_planner_workset.rs` proves config-backed
    `docs_planner_workset_from_config(...)` over the external Modelica path,
    so the additive proof now covers docs-facing deterministic planner
    workset shaping over a stable external-plugin repository as well
82. that same `M6` additive slice now reaches the docs-facing planner-rank
    peer too: `tests/integration/docs_planner_rank.rs` proves config-backed
    `docs_planner_rank_from_config(...)` over the external Modelica path, so
    the additive proof now covers docs-facing deterministic planner ranking
    over a stable external-plugin repository as well
83. that same `M6` additive slice now reaches the docs-facing planner-item
    peer too: `tests/integration/docs_planner_item.rs` proves config-backed
    `docs_planner_item_from_config(...)` over the external Modelica path, so
    the additive proof now covers docs-facing deterministic planner item
    reopening over a stable external-plugin repository as well
84. that same `M6` additive slice now reaches the docs-facing planner-search
    peer too: `tests/integration/docs_planner_search.rs` proves config-backed
    `docs_planner_search_from_config(...)` over the external Modelica path,
    so the additive proof now covers docs-facing deterministic planner search
    over a stable external-plugin repository as well
85. that same `M6` additive slice now reaches the Studio docs route layer
    too: the `studio_repo_sync_api` lib-test module now proves
    `/api/docs/planner-search` over the external Modelica plugin path, so the
    additive proof is no longer limited to analyzer-entry consumers; the same
    slice also remounts `tests/support/repo_fixture.rs` next to
    `repo_intelligence.rs` inside `src/analyzers/service/projection/tests.rs`
    so the shared lib-test projection fixture path keeps compiling after the
    test-support topology cleanup
86. that same `M6` additive slice now reaches a second Studio docs route peer
    too: the `studio_repo_sync_api` lib-test module now proves
    `/api/docs/planner-item` over the external Modelica plugin path by
    reopening a stable gap id sourced from the docs-facing projected-gap
    report, so the gateway-layer additive proof now covers deterministic
    planner-gap reopening as well as planner search
87. that same `M6` additive slice now reaches a third Studio docs route peer
    too: the `studio_repo_sync_api` lib-test module now proves
    `/api/docs/planner-workset` over the external Modelica plugin path by
    filtering the selection onto the injected `NoDocs` reference gap, so the
    gateway-layer additive proof now covers deterministic planner-workset
    shaping as well as planner search and planner-item reopening
88. that same `M6` additive slice now reaches a fourth Studio docs route peer
    too: the `studio_repo_sync_api` lib-test module now proves
    `/api/docs/planner-rank` over the external Modelica plugin path by
    filtering the selection onto the injected `NoDocs` reference gap, so the
    gateway-layer additive proof now covers deterministic planner ranking as
    well as planner search, planner-item reopening, and planner-workset
    shaping
89. that same `M6` additive slice now reaches a fifth Studio docs route peer
    too: the `studio_repo_sync_api` lib-test module now proves
    `/api/docs/planner-queue` over the external Modelica plugin path by
    filtering the selection onto the injected `NoDocs` reference gap, so the
    gateway-layer additive proof now covers deterministic planner queue
    shaping as well as planner search, planner-item reopening, planner-
    workset shaping, and planner ranking
90. that same `M6` additive slice now exits the Studio planner subtree too:
    the `studio_repo_sync_api` lib-test module now proves `/api/docs/search`
    over the external Modelica plugin path, so the gateway-layer additive
    proof now reaches the first non-planner docs-facing route family as well
91. that same `M6` additive slice now extends the non-planner Studio docs
    route family too: the `studio_repo_sync_api` lib-test module now proves
    `/api/docs/retrieval` over the external Modelica plugin path, so the
    gateway-layer additive proof now covers mixed docs-facing retrieval as
    well as plain docs search
92. that same `M6` additive slice now pushes one level deeper into the
    non-planner Studio docs route family: the `studio_repo_sync_api`
    lib-test module now proves `/api/docs/retrieval-context` over the
    external Modelica plugin path, so the gateway-layer additive proof now
    covers deterministic node-context reopening as well as mixed retrieval
    and plain docs search
93. that same `M6` additive slice now closes the sibling deterministic
    reopening peer too: the `studio_repo_sync_api` lib-test module now
    proves `/api/docs/retrieval-hit` over the external Modelica plugin
    path, so the gateway-layer additive proof now covers deterministic hit
    reopening as well as node-context reopening, mixed retrieval, and plain
    docs search
94. that same `M6` additive slice now closes the deterministic page-lookup
    peer too: the `studio_repo_sync_api` lib-test module now proves
    `/api/docs/page` over the external Modelica plugin path, so the
    gateway-layer additive proof now covers deterministic docs page lookup
    alongside hit reopening, node-context reopening, mixed retrieval, and
    plain docs search
95. that same `M6` additive slice now closes the deterministic family-context
    peer too: the `studio_repo_sync_api` lib-test module now proves
    `/api/docs/family-context` over the external Modelica plugin path, so
    the gateway-layer additive proof now covers grouped family-context
    reopening alongside page lookup, hit reopening, node-context reopening,
    mixed retrieval, and plain docs search
96. that same `M6` additive slice now closes the deterministic family-search
    peer too: the `studio_repo_sync_api` lib-test module now proves
    `/api/docs/family-search` over the external Modelica plugin path, so
    the gateway-layer additive proof now covers grouped family-search
    expansion alongside family-context reopening, page lookup, hit
    reopening, node-context reopening, mixed retrieval, and plain docs
    search
97. that same `M6` additive slice now closes the deterministic family-cluster
    peer too: the `studio_repo_sync_api` lib-test module now proves
    `/api/docs/family-cluster` over the external Modelica plugin path, so
    the gateway-layer additive proof now covers single-family reopening
    alongside family-search expansion, family-context reopening, page
    lookup, hit reopening, node-context reopening, mixed retrieval, and
    plain docs search
98. that same `M6` additive slice now closes the deterministic navigation
    peer too: the `studio_repo_sync_api` lib-test module now proves
    `/api/docs/navigation` over the external Modelica plugin path, so the
    gateway-layer additive proof now covers tree-context plus family-cluster
    reopening alongside single-family reopening, family-search expansion,
    family-context reopening, page lookup, hit reopening, node-context
    reopening, mixed retrieval, and plain docs search
99. that same `M6` additive slice now closes the deterministic navigation-
    search peer too: the `studio_repo_sync_api` lib-test module now proves
    `/api/docs/navigation-search` over the external Modelica plugin path, so
    the gateway-layer additive proof now covers grouped navigation-bundle
    expansion alongside deterministic navigation, single-family reopening,
    family-search expansion, family-context reopening, page lookup, hit
    reopening, node-context reopening, mixed retrieval, and plain docs
    search
100. that same `M6` additive slice now closes the docs projected-gap-report
     peer too: the `studio_repo_sync_api` lib-test module now proves
     `/api/docs/projected-gap-report` over the external Modelica plugin
     path, so the gateway-layer additive proof now covers docs-facing gap
     reporting alongside grouped navigation-bundle expansion, deterministic
     navigation, single-family reopening, family-search expansion,
     family-context reopening, page lookup, hit reopening, node-context
     reopening, mixed retrieval, and plain docs search
101. that same `M6` additive slice now exits the Studio docs route family
     and opens the sibling Studio repo route family too: the
     `studio_repo_sync_api` lib-test module now proves
     `/api/repo/overview` over the external Modelica plugin path, so the
     gateway-layer additive proof now covers repo-summary reopening at the
     same outward layer as well
102. that same `M6` additive slice now closes the sibling Studio repo
     module-search peer too: the `studio_repo_sync_api` lib-test module now
     proves `/api/repo/module-search` over the external Modelica plugin
     path, so the gateway-layer additive proof now covers deterministic
     module-search reopening at that same outward layer as well
103. that same `M6` additive slice now closes the sibling Studio repo
     symbol-search peer too: the `studio_repo_sync_api` lib-test module now
     proves `/api/repo/symbol-search` over the external Modelica plugin
     path, so the gateway-layer additive proof now covers deterministic
     symbol-search reopening at that same outward layer as well
104. that same `M6` additive slice now closes the sibling Studio repo
     example-search peer too: the `studio_repo_sync_api` lib-test module now
     proves `/api/repo/example-search` over the external Modelica plugin
     path, so the gateway-layer additive proof now covers deterministic
     example-search reopening at that same outward layer as well
105. that same `M6` additive slice now closes the sibling Studio repo
     doc-coverage peer too: the `studio_repo_sync_api` lib-test module now
     proves `/api/repo/doc-coverage` over the external Modelica plugin
     path, so the gateway-layer additive proof now covers deterministic
     module-scoped doc-coverage reopening at that same outward layer as well
106. that same `M6` additive slice now pushes through the sibling Studio repo
     lifecycle-and-projection peers as a batch: the
     `studio_repo_sync_api` lib-test module now proves `/api/repo/sync`,
     `/api/repo/projected-pages`, and `/api/repo/projected-gap-report` over
     the external Modelica plugin path, so the gateway-layer additive proof
     now covers repo status reopening, projected-page enumeration, and
     projected-gap reporting at that same outward layer as well
107. that same `M6` additive slice now pushes through the deterministic
     sibling Studio repo projected reopen family as a batch too: the
     `studio_repo_sync_api` lib-test module now proves
     `/api/repo/projected-page`, `/api/repo/projected-page-index-tree`,
     `/api/repo/projected-page-index-node`, `/api/repo/projected-retrieval-hit`,
     and `/api/repo/projected-retrieval-context` over the external Modelica
     plugin path, so the gateway-layer additive proof now covers stable
     symbol-page reopening, tree reopening, node reopening, deterministic hit
     reopening, and node-context reopening at that same outward layer as well
108. that same `M6` additive slice now closes the remaining sibling Studio
     repo projected query-and-navigation family as a batch too: the
     `studio_repo_sync_api` lib-test module now proves
     `/api/repo/projected-page-index-tree-search`,
     `/api/repo/projected-page-search`, `/api/repo/projected-retrieval`,
     `/api/repo/projected-page-family-context`,
     `/api/repo/projected-page-family-search`,
     `/api/repo/projected-page-family-cluster`,
     `/api/repo/projected-page-navigation`,
     `/api/repo/projected-page-navigation-search`, and
     `/api/repo/projected-page-index-trees` over the external Modelica
     plugin path, so the gateway-layer additive proof now also covers
     deterministic section search, projected page search, mixed projected
     retrieval, family-context reopening, family-search expansion,
     single-family reopening, navigation-bundle reopening,
     navigation-search expansion, and projected tree listing at that same
     outward layer as well

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
