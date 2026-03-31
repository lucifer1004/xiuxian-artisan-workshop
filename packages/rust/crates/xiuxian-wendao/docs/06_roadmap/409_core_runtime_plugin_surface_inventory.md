# Wendao Core Runtime Plugin Surface Inventory

:PROPERTIES:
:ID: wendao-core-runtime-plugin-surface-inventory
:PARENT: [[index]]
:TAGS: roadmap, migration, plugins, core, runtime, julia, inventory
:STATUS: ACTIVE
:END:

## Mission

This note started as the early `P0` ownership map for the Wendao
`core` / `runtime` / plugin-package migration. It is now maintained as the
active late-`M6` outward-surface inventory and compatibility ledger.

Primary references:

- `[[docs/rfcs/2026-03-27-wendao-core-runtime-plugin-migration-rfc.md]]`
- `[[docs/rfcs/2026-03-27-wendao-arrow-plugin-flight-rfc.md]]`
- `[[.data/blueprints/wendao_arrow_plugin_core_runtime_migration.md]]`
- `[[06_roadmap/405_large_rust_modularization]]`
- `[[06_roadmap/410_p1_generic_plugin_contract_staging]]`

This document now records the live outward surfaces, the remaining
package-owned compatibility seams, and the additive external-plugin proof
coverage that currently governs the migration.

## Gate Intent

The active alignment bundle requires:

1. an accurate inventory of the live outward surfaces
2. one explicit record of which compatibility seams are still intentionally
   package-owned
3. one outward-facing summary of the late-`M6` additive proof
4. active documentation that no longer describes the tree as early-phase
   extraction startup

The historical ownership map remains below, but the document's primary job is
now to keep late-`M6` outward reality synchronized across the active docs.

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

## Late-M6 Outward Surface Summary

The current live outward surfaces are:

| Outward family                        | Canonical surface                                                                                              | Current late-`M6` status                                                                                            | Compatibility note                                                   |
| :------------------------------------ | :------------------------------------------------------------------------------------------------------------- | :------------------------------------------------------------------------------------------------------------------ | :------------------------------------------------------------------- |
| Generic artifact inspection           | `wendao.plugin_artifact`, `GET /api/ui/plugins/{plugin_id}/artifacts/{artifact_id}`                            | canonical and live                                                                                                  | Julia-named outward artifact surfaces are retired from the host      |
| Studio docs gateway family            | `/api/docs/*`                                                                                                  | external Modelica proof now covers docs-facing search, retrieval, navigation, family, gap-report, and planner peers | no host-owned language-specific route family remains in this surface |
| Studio repo query/projection family   | `/api/repo/overview`, `/api/repo/*search`, `/api/repo/doc-coverage`, `/api/repo/sync`, `/api/repo/projected-*` | external Modelica proof now covers repo-facing query, projection, reopen, and navigation peers                      | additive proof stays on generic host surfaces                        |
| Studio repo service-state family      | `POST /api/repo/index`, `GET /api/repo/index/status`                                                           | `Stage A` is complete; the external Modelica path now covers the remaining service-state bundle                     | bounded local helper reuse only; no dead-code suppressions added     |
| Remaining Julia compatibility imports | `xiuxian_wendao_julia::compatibility::link_graph::*`                                                           | package-owned only                                                                                                  | host crate-root compatibility shims are retired                      |

The current late-`M6` outward story is therefore:

1. generic outward artifact inspection is canonical
2. repo-facing, docs-facing, and Studio-facing additive proof now all exist on
   one non-Julia external plugin path
3. the `M6` additive-proof exit condition is now satisfied
4. `Phase 7: Flight-First Runtime Negotiation` is now complete
5. `Phase-8 Stage A: Tooling Reality Inventory Bundle` is now complete
6. `Phase-8 Stage B: Contract and Dependency Policy Bundle` is now complete
7. `Phase-8 Stage C: Lane Integration and Gate Bundle` is now complete
8. the explicit `Phase 8` gate decision is `go`
9. the next governed move is no longer more outward-surface expansion inside
   this inventory; it should move to a new macro-phase proposal or a bounded
   remediation plan for the surfaced advisory findings

## Phase-7 Transport Surface Inventory

The `Phase-7 Stage A` inventory resolves the live transport surface as
follows:

| Surface                              | Current owner                 | Stage-A finding                                                                                                                                                             | Phase-7 implication                                                                                                                                      |
| :----------------------------------- | :---------------------------- | :-------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | :------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Generic transport contract           | `xiuxian-wendao-core`         | `PluginCapabilityBinding` carries `endpoint`, `transport`, and `contract_version`; `PluginTransportKind` declares `ArrowFlight`, `ArrowIpcHttp`, and `LocalProcessArrowIpc` | negotiation policy should stay generic and contract-first                                                                                                |
| Runtime transport construction       | `xiuxian-wendao-runtime`      | `src/transport/client.rs` owns `build_arrow_transport_client_from_binding(...)`, which currently materializes `ArrowIpcHttp` only                                           | `Stage B` should add preference/selection logic here instead of in host gateway code                                                                     |
| Host Arrow response encoding         | `xiuxian-wendao` host gateway | `src/gateway/studio/search/handlers/arrow_transport.rs` only serializes local Arrow payload responses                                                                       | this seam is not the negotiation owner and should stay out of transport-policy growth                                                                    |
| Outward transport inspection payload | `xiuxian-wendao` Studio types | `UiPluginArtifact` now exposes `base_url`, `route`, `health_route`, `timeout_secs`, `schema_version`, `selected_transport`, `fallback_from`, and `fallback_reason`         | `Stage C` is complete for the outward inspection family                                                                                                   |

The canonical Phase-7 transport preference order is now fixed as:

1. `ArrowFlight`
2. `ArrowIpcHttp`
3. `LocalProcessArrowIpc`

Interpretation:

1. `ArrowFlight` is the preferred runtime negotiation result where a provider
   and host path can support it
2. `ArrowIpcHttp` is the bounded compatibility fallback
3. `LocalProcessArrowIpc` remains reserved for explicitly managed local
   provider paths rather than generic remote negotiation

Current Stage-B implementation status:

1. the runtime-owned negotiation policy now lives in
   `packages/rust/crates/xiuxian-wendao-runtime/src/transport/negotiation.rs`
2. the current rerank transport path now delegates through that negotiation
   seam instead of directly materializing an Arrow IPC client
3. the runtime now also owns a real Flight client seam in
   `packages/rust/crates/xiuxian-wendao-runtime/src/transport/flight.rs`
4. that Flight client intentionally uses `arrow-flight = 57.3.0` so the data
   plane stays aligned with the LanceDB Arrow line already present in
   `lance-arrow`
5. the host-side rerank path stays on the current engine Arrow line by
   reusing `xiuxian-vector`'s existing
   `engine_batches_to_lance_batches(...)` and
   `lance_batches_to_engine_batches(...)` conversion seam
6. when both `ArrowFlight` and `ArrowIpcHttp` candidates are present, the
   runtime now selects supported `ArrowFlight` bindings first and falls back
   deterministically to `ArrowIpcHttp` only when the preferred binding is
   incomplete
7. `Phase-7 Stage B` is complete: runtime policy and Flight materialization
   now live on the LanceDB Arrow line
8. `Phase-7 Stage C` is complete: outward inspection payloads and rerank
   diagnostics now surface negotiated transport and fallback decisions
9. the explicit `Phase 7` gate decision is `go`

## Phase-8 Contract and Dependency Policy Snapshot

`Phase-8 Stage B` fixes the first governance boundaries as:

1. workspace security remains one blocking lane family through the existing
   `cargo-audit` and `cargo-deny` baseline
2. the current `cargo-deny` contract remains pragmatic:
   advisories, bans, and sources are blocking, while duplicate-version
   findings stay warn-only and license gating stays out of the initial
   bounded rollout
3. semver governance is now explicitly owned by `xiuxian-wendao-core`; the
   initial blocking semver lane should target that crate only
4. `xiuxian-wendao`, `xiuxian-wendao-runtime`, `xiuxian-wendao-julia`, and
   `xiuxian-wendao-modelica` are not part of the initial semver gate because
   their public surfaces are still migration-owned
5. dependency-hygiene rollout stays advisory-first:
   `cargo-machete` should target the Wendao migration cluster, and
   `cargo-udeps` should start with `xiuxian-wendao-core` plus
   `xiuxian-wendao-runtime`

Current Stage-C status:

1. the bounded governance bundle is now physically wired through `justfile`,
   `nix/modules/rust.nix`, `nix/modules/tasks.nix`, and the main Rust CI
   workflows
2. the blocking semver lane for `xiuxian-wendao-core` is live and passed on
   the current tree
3. the advisory `cargo-machete` lane is live and now passes cleanly on the
   current Wendao migration cluster after removing the stale
   `xiuxian-wendao` dependency entries
4. the advisory `cargo-udeps` lane is wired with bounded skip semantics when
   `rustup` nightly is unavailable in the current Nix-managed environment
5. follow-on bounded sibling remediations removed the stale
   `xiuxian-llm` direct dependencies `rmcp`, `fast_image_resize`, and
   `spider_agent`, then retired the dead `xiuxian-zhenfa`
   `tests/support/gateway.rs` helper; `toml` and `xiuxian-config-core`
   stay declared in `xiuxian-llm` as explicit `cargo-machete`-ignored macro
   dependencies, and live `rmcp` ownership is now concentrated in
   `xiuxian-daochang`
6. a further bounded contraction moved `xiuxian-daochang` production callers
   off `rmcp` response/request model types, so direct `rmcp` usage in
   `src/` is now narrowed to `tool_runtime/bridge.rs`; full
   `xiuxian-daochang` crate verification remains blocked by unrelated
   pre-existing module/export failures outside the touched seam
7. the final bounded production-side replacement moved
   `xiuxian-daochang/src/tool_runtime/bridge.rs` off `rmcp` entirely by
   landing a self-owned streamable-HTTP JSON-RPC client for `initialize`,
   `tools/list`, and `tools/call`, and by moving `rmcp` to
   `dev-dependencies` for the test-side server harness only; direct `rmcp`
   usage no longer exists in `xiuxian-daochang/src/`
8. the bounded follow-on advisory cleanup has now removed the stale
   `xiuxian-daochang` direct dependencies `comrak` and `regex`, so the
   package is clean under the current `cargo-machete` lane even though full
   crate verification is still blocked by unrelated pre-existing failures
9. the next bounded crate-health slice has repaired the stale
   `system_prompt_injection_state` owner drift in `xiuxian-daochang`, so the
   compile front now advances past that session prompt-injection seam and
   stops in other pre-existing import/export clusters instead
10. the following bounded crate-health slice has repaired the
    `runtime_agent_factory` test-support visibility seam, so that private
    helper-import cluster no longer appears at the compile front either
11. the next bounded Discord crate-health slice has removed the dead mounted
    `channels/discord/channel/constructors.rs` duplicate, so that stale
    `omni_agent` / `ChannelCommandRequest` constructor drift is no longer
    carried inside the live Discord channel module tree
12. the following bounded channel-runtime slice has restored the missing
    `nodes/channel/common.rs` embedding-memory guard helper, so the live
    `nodes/channel/{discord,telegram}.rs` launch paths no longer depend on a
    nonexistent shared leaf and the compile front now advances into deeper
    Discord runtime dispatch and Telegram channel wiring drift
13. the next bounded Discord runtime slice has now repaired the stale
    `channels/discord/runtime/dispatch/` surface by restoring child-module
    wiring, a local `dispatch/support.rs` logging/preemption leaf, the
    `process_discord_message(...)` compatibility wrapper, crate-internal
    `ForegroundInterruptController` visibility, and the shared
    `compose_turn_content(...)` helper in `channels/managed_runtime/turn.rs`;
    that Discord dispatch cluster no longer appears at the compile front,
    which now starts deeper in Telegram runtime surface drift and other
    pre-existing export failures
14. the following bounded channel path-normalization slice has now repaired
    the remaining live Telegram channel/runtime owner drift by mounting the
    existing `channel/outbound_text.rs` leaf, re-exporting the
    `jobs::{JobRecord, QueuedJob, epoch_millis}` owner seam, switching the
    touched imports from `super::super::...` to `crate::...`, and retiring
    dead duplicate Discord managed-session `admin` / `injection` leaves plus
    the dead Telegram `session_context` duplicate; the
    `xiuxian-daochang` compile front no longer stops in that channel family
    and now exits the crate at a pre-existing `xiuxian-wendao` transport
    import failure
15. the next bounded runtime transport slice has now repaired that
    `xiuxian-wendao` import failure by restoring the missing
    `RepoSearchFlightRouteProvider` re-export in
    `xiuxian-wendao-runtime/src/transport/mod.rs`; the compile front no
    longer exits `xiuxian-daochang` at that transport seam
16. the following bounded helper slice has now repaired the
    `xiuxian-daochang` `jobs/heartbeat` owner import and the initial
    embedding helper drift by replacing the dead `env_first_non_empty`
    macro dependency with a local env scan and by making the currently
    unwired `mistral_sdk` embedding path fail explicitly instead of failing
    the build; the current compile front now starts deeper in `llm/*`,
    `resolve.rs`, and `runtime_agent_factory/*`
17. the following bounded crate-health slice has now repaired the
    `resolve.rs` plus `runtime_agent_factory/*` owner drift by moving the
    channel runtime enums into a crate-owned `channel_runtime.rs` seam,
    rebinding factory imports to crate-owned settings/config owners,
    restoring the missing env-parse helpers, and replacing touched relative
    paths with `crate::...`; the current compile front now starts deeper in
    `session/*`, `test_support/*`, `lib.rs`, and `agent/*` private-module
    exposure consumed by `test_support/*`
18. the following bounded crate-health slice has now repaired the
    `session/*` owner drift by restoring the missing local `SessionStore`,
    switching the live `TurnSlot` owner back to `xiuxian_window`, fixing the
    `RedisSessionBackend` message-content snapshot field, and rebinding the
    touched session tests to `xiuxian_daochang`; the current compile front
    now starts deeper in `test_support/*`, `lib.rs`, and `agent/*`
    private-module exposure consumed by `test_support/*`

## Historical Ownership Inventory

The table below remains as the resolved extraction-era ownership map that got
the program to the current late-`M6` shape.

| Current surface                                                                                                                                                                                              | Current path                                                                                                                                                   | Current problem                                                                                                                                                                                                           | Target owner                                                      | Target namespace                                                                                                                                                   | Planned phase    |
| :----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | :------------------------------------------------------------------------------------------------------------------------------------------------------------- | :------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ | :---------------------------------------------------------------- | :----------------------------------------------------------------------------------------------------------------------------------------------------------------- | :--------------- |
| Julia-specific runtime config records such as `LinkGraphJuliaRerankRuntimeConfig`, `LinkGraphJuliaAnalyzerServiceDescriptor`, `LinkGraphJuliaAnalyzerLaunchManifest`, and `LinkGraphJuliaDeploymentArtifact` | `src/link_graph/runtime_config/models.rs`                                                                                                                      | Stable runtime config is encoded as Julia-only types inside the host                                                                                                                                                      | `runtime` and `xiuxian-wendao-julia`                              | `runtime/runtime_config/providers/`, `runtime/artifacts/`, `xiuxian-wendao-julia/capabilities/`, `xiuxian-wendao-julia/artifacts/`, `xiuxian-wendao-julia/launch/` | `P1`, `P3`, `P4` |
| `link_graph.retrieval.julia_rerank` host config path                                                                                                                                                         | `src/link_graph/runtime_config/models.rs`, `src/link_graph/runtime_config/resolve/policy.rs`                                                                   | Provider identity is hardcoded into the config shape, which blocks generic capability routing                                                                                                                             | `runtime`                                                         | `runtime/runtime_config/capabilities/`, `runtime/negotiation/`                                                                                                     | `P1`, `P3`       |
| Julia-specific environment variables and launcher defaults such as `XIUXIAN_WENDAO_LINK_GRAPH_JULIA_RERANK_*` and `.data/WendaoAnalyzer/scripts/run_analyzer_service.sh`                                     | `src/link_graph/runtime_config/constants.rs`                                                                                                                   | Host runtime defaults are language-scoped rather than provider-scoped                                                                                                                                                     | `runtime` with Julia-owned defaults in plugin package             | `runtime/runtime_config/providers/`, `runtime/launch/`, `xiuxian-wendao-julia/manifest/`, `xiuxian-wendao-julia/launch/`                                           | `P1`, `P3`, `P4` |
| Builtin Julia registration in host bootstrap                                                                                                                                                                 | `src/analyzers/service/bootstrap.rs`                                                                                                                           | The host owns Julia plugin assembly directly instead of loading a package-defined provider                                                                                                                                | `runtime` and `xiuxian-wendao-julia`                              | `runtime/registry/`, `runtime/discovery/`, `xiuxian-wendao-julia/entry/`                                                                                           | `P3`, `P4`       |
| Former sibling-source inclusion hacks for package-local plugin code                                                                                                                                          | `src/analyzers/languages/mod.rs`                                                                                                                               | Julia and Modelica previously entered the host through `#[path]`; the current tree now uses normal crate dependencies for both, so this row remains only as a resolved retirement checkpoint for future plugin onboarding | resolved for Julia and Modelica                                   | `xiuxian-wendao-julia/plugin/`, `xiuxian-wendao-modelica/plugin/`, and package dependency registration instead of source inclusion                                 | `P4`, `P6`       |
| Julia-specific rerank planning and transport helpers                                                                                                                                                         | `src/link_graph/index/search/plan/payload/quantum.rs`                                                                                                          | Capability execution path is hardcoded to Julia rather than routed through a generic provider binding                                                                                                                     | `runtime` with Julia-specific transport details in plugin package | `runtime/capabilities/rerank/`, `runtime/transport/`, `runtime/negotiation/`, `xiuxian-wendao-julia/transport/`, `xiuxian-wendao-julia/capabilities/rerank/`       | `P1`, `P3`, `P4` |
| Julia-specific request-batch builder names in ignition helpers                                                                                                                                               | `src/link_graph/index/search/quantum_fusion/openai_ignition.rs`, `src/link_graph/index/search/quantum_fusion/vector_ignition.rs`                               | Shared preparation logic is named after one provider even though the long-term host contract is capability-oriented                                                                                                       | `runtime`                                                         | `runtime/capabilities/rerank/request/`                                                                                                                             | `P1`, `P3`       |
| Link-graph public re-exports of Julia-specific types                                                                                                                                                         | `src/link_graph/mod.rs`, `src/link_graph/runtime_config.rs`                                                                                                    | The link-graph domain surface leaks one plugin provider as core vocabulary                                                                                                                                                | `core` compatibility shim plus `runtime` implementation           | `core/capabilities/`, `runtime/capabilities/`, `runtime/artifacts/`                                                                                                | `P1`, `P2`, `P5` |
| Julia-specific test fixtures and planned integration tests                                                                                                                                                   | `tests/integration/planned_search_julia_rerank*.rs`, `tests/integration/support/wendaoarrow_official_examples.rs`, `src/gateway/studio/router/tests/config.rs` | The test topology mirrors the current host leak and must migrate alongside the runtime and plugin seams                                                                                                                   | split across `runtime` and `xiuxian-wendao-julia`                 | `runtime/tests/capabilities/rerank/`, `runtime/tests/artifacts/`, `xiuxian-wendao-julia/tests/launch/`, `xiuxian-wendao-julia/tests/artifacts/`                    | `P3`, `P4`, `P5` |

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
- the `M5` outward artifact cutover is now complete on the Studio/OpenAPI
  side: `UiPluginArtifact` is the primary payload, the canonical schema
  export is generic-only, and
  `/api/ui/plugins/{plugin_id}/artifacts/{artifact_id}` is now the only live
  Studio/OpenAPI UI artifact endpoint
- the old Studio Julia compatibility path is now fully retired from code:
  `UiJuliaDeploymentArtifact`, the dedicated compatibility type leaf, the
  route-local compat adapter, `JuliaDeploymentArtifactQuery`,
  `get_julia_deployment_artifact`, and
  `GET /api/ui/julia-deployment-artifact` are all gone from the live tree
- the OpenAPI Julia route-path aliases are now retired from code too:
  `API_UI_JULIA_DEPLOYMENT_ARTIFACT_*` are gone, and the route inventory now
  validates only the canonical plugin-artifact path
- the `M5` outward artifact cutover is now complete on the Zhenfa side too:
  `wendao.plugin_artifact` is the only live tool/RPC artifact surface, and
  the former Julia outward tool name, compat-specific tool/RPC path, native
  compatibility helper folder, and Julia helper/type aliases are all retired
  from code
- the crate-root and `runtime_config` top-level Julia-named DTO/helper exports
  are now retired too: those names live only under the explicit compatibility
  namespaces instead of leaking through flat public re-export blocks
- the former host crate-root compatibility shim is now retired from code too:
  `src/compatibility/julia.rs`, `src/compatibility/link_graph.rs`, and the
  `pub mod compatibility;` mount in `src/lib.rs` are all gone, so the touched
  internal consumers now read Julia compatibility imports directly from
  `packages/rust/crates/xiuxian-wendao-julia/src/compatibility/link_graph/`
- the first `M6` additive-plugin proof is now landed too:
  `xiuxian-wendao-modelica` consumes
  `xiuxian-wendao-core::repo_intelligence` for production contracts, the host
  loads Modelica through a normal optional crate dependency, and Modelica
  keeps `xiuxian-wendao` only as a dev-dependency for registry-aware
  integration-query validation
- that same `M6` slice now has a host-side proof too:
  `packages/rust/crates/xiuxian-wendao/tests/integration/repo_overview.rs`
  now exercises the external Modelica plugin through the builtin registry and
  the shared repo-overview/module-search/example-search entry points
- that `M6` host proof is now deeper than the first search slice:
  `repo_symbol_search.rs`, `repo_relations.rs`, and `repo_projected_page.rs`
  now also exercise the external Modelica plugin through builtin-registry
  symbol-search, relation-graph, and projected-page lookup consumers
- that same `M6` host proof now reaches parsed page hierarchy too:
  `repo_projected_page_index_tree.rs` exercises the external Modelica plugin
  through builtin-registry projected page-index tree generation and lookup
- that same `M6` host proof now reaches stable node addressing too:
  `repo_projected_page_index_node.rs` exercises the external Modelica plugin
  through builtin-registry projected page-index node lookup
- that same `M6` host proof now reaches assembled navigation too:
  `repo_projected_page_navigation.rs` exercises the external Modelica plugin
  through builtin-registry projected page navigation bundles
- that same `M6` host proof now reaches grouped family context too:
  `repo_projected_page_family_context.rs` exercises the external Modelica
  plugin through builtin-registry projected page-family context lookup
- that same `M6` host proof now reaches singular family-cluster lookup too:
  `repo_projected_page_family_cluster.rs` exercises the external Modelica
  plugin through builtin-registry projected page-family cluster lookup
- that same `M6` host proof now reaches search-driven family expansion too:
  `repo_projected_page_family_search.rs` exercises the external Modelica
  plugin through builtin-registry projected page-family search
- that same `M6` host proof now reaches search-driven navigation expansion too:
  `repo_projected_page_navigation_search.rs` exercises the external Modelica
  plugin through builtin-registry projected page-navigation search
- that same `M6` additive slice now reaches a docs-facing search consumer too:
  `docs_navigation_search.rs` exercises the external Modelica plugin through
  builtin-registry docs-facing projected page-navigation search
- that same `M6` additive slice now reaches the docs-facing family-search
  peer too: `docs_family_search.rs` exercises the external Modelica plugin
  through builtin-registry docs-facing projected page-family search
- that same `M6` additive slice now reaches the docs-facing family-context
  peer too: `docs_family_context.rs` exercises the external Modelica plugin
  through builtin-registry docs-facing projected page-family context
- that same `M6` additive slice now reaches the docs-facing navigation lookup
  peer too: `docs_navigation.rs` exercises the external Modelica plugin
  through builtin-registry docs-facing projected page navigation lookup
- that same `M6` additive slice now reaches the docs-facing family-cluster
  peer too: `docs_family_cluster.rs` exercises the external Modelica plugin
  through builtin-registry docs-facing projected page-family cluster lookup
- that same `M6` additive slice now reaches the docs-facing page lookup peer
  too: `docs_page.rs` exercises the external Modelica plugin through
  builtin-registry docs-facing projected page lookup
- that same `M6` additive slice now reaches the docs-facing page-index tree
  lookup peer too: `docs_page_index_tree.rs` exercises the external Modelica
  plugin through builtin-registry docs-facing projected page-index tree
  lookup
- that same `M6` additive slice now reaches the docs-facing page-index node
  lookup peer too: `docs_page_index_node.rs` exercises the external Modelica
  plugin through builtin-registry docs-facing projected page-index node
  lookup
- that same `M6` additive slice now reaches the docs-facing page-index tree
  search peer too: `docs_page_index_tree_search.rs` exercises the external
  Modelica plugin through builtin-registry docs-facing projected page-index
  tree search
- that same `M6` additive slice now reaches the docs-facing page-index trees
  peer too: `docs_page_index_trees.rs` exercises the external Modelica plugin
  through builtin-registry docs-facing projected page-index tree listing
- that same `M6` additive slice now reaches the docs-facing page-index
  documents peer too: `docs_page_index_documents.rs` exercises the external
  Modelica plugin through builtin-registry docs-facing projected page-index
  document generation
- that same `M6` additive slice now reaches the docs-facing markdown
  documents peer too: `docs_markdown_documents.rs` exercises the external
  Modelica plugin through builtin-registry docs-facing projected markdown
  document generation
- that same `M6` additive slice now reaches the docs-facing search peer too:
  `docs_search.rs` exercises the external Modelica plugin through
  builtin-registry docs-facing projected page search
- that same `M6` additive slice now reaches the docs-facing retrieval peer
  too: `docs_retrieval.rs` exercises the external Modelica plugin through
  builtin-registry docs-facing mixed projected retrieval
- that same `M6` additive slice now reaches the docs-facing retrieval-
  context peer too: `docs_retrieval_context.rs` exercises the external
  Modelica plugin through builtin-registry docs-facing local projected
  retrieval context
- that same `M6` additive slice now reaches the docs-facing retrieval-hit
  peer too: `docs_retrieval_hit.rs` exercises the external Modelica plugin
  through builtin-registry docs-facing deterministic projected retrieval-hit
  reopening
- that same `M6` additive slice now reaches the docs-facing projected-gap
  report peer too: `docs_projected_gap_report.rs` exercises the external
  Modelica plugin through builtin-registry docs-facing projected gap
  reporting
- that same `M6` additive slice now reaches the docs-facing planner-queue
  peer too: `docs_planner_queue.rs` exercises the external Modelica plugin
  through builtin-registry docs-facing deterministic planner queue shaping
- that same `M6` additive slice now reaches the docs-facing planner-workset
  peer too: `docs_planner_workset.rs` exercises the external Modelica plugin
  through builtin-registry docs-facing deterministic planner workset shaping
- that same `M6` additive slice now reaches the docs-facing planner-rank
  peer too: `docs_planner_rank.rs` exercises the external Modelica plugin
  through builtin-registry docs-facing deterministic planner ranking
- that same `M6` additive slice now reaches the docs-facing planner-item
  peer too: `docs_planner_item.rs` exercises the external Modelica plugin
  through builtin-registry docs-facing deterministic planner item reopening
- that same `M6` additive slice now reaches the docs-facing planner-search
  peer too: `docs_planner_search.rs` exercises the external Modelica plugin
  through builtin-registry docs-facing deterministic planner search
- that same `M6` additive slice now reaches the Studio docs route layer too:
  `tests/unit/studio_repo_sync_api.rs` exercises `/api/docs/planner-search`
  through the builtin-registry external Modelica path, so the additive proof
  now covers one real gateway consumer above the analyzer entrypoint
- that same `M6` additive slice now reaches a second Studio docs route peer
  too: `tests/unit/studio_repo_sync_api.rs` also exercises
  `/api/docs/planner-item` through the builtin-registry external Modelica
  path, so the gateway-layer additive proof now covers deterministic planner
  gap reopening as well as planner search
- that same `M6` additive slice now reaches a third Studio docs route peer
  too: `tests/unit/studio_repo_sync_api.rs` also exercises
  `/api/docs/planner-workset` through the builtin-registry external Modelica
  path, so the gateway-layer additive proof now covers deterministic planner
  workset shaping as well as planner search and planner-item reopening
- that same `M6` additive slice now reaches a fourth Studio docs route peer
  too: `tests/unit/studio_repo_sync_api.rs` also exercises
  `/api/docs/planner-rank` through the builtin-registry external Modelica
  path, so the gateway-layer additive proof now covers deterministic planner
  ranking as well as planner search, planner-item reopening, and
  planner-workset shaping
- that same `M6` additive slice now reaches a fifth Studio docs route peer
  too: `tests/unit/studio_repo_sync_api.rs` also exercises
  `/api/docs/planner-queue` through the builtin-registry external Modelica
  path, so the gateway-layer additive proof now covers deterministic planner
  queue shaping as well as planner search, planner-item reopening,
  planner-workset shaping, and planner ranking
- that same `M6` additive slice now exits the Studio planner subtree too:
  `tests/unit/studio_repo_sync_api.rs` also exercises `/api/docs/search`
  through the builtin-registry external Modelica path, so the gateway-layer
  additive proof now reaches the first non-planner docs-facing route family
  as well
- that same `M6` additive slice now extends the non-planner Studio docs
  route family too: `tests/unit/studio_repo_sync_api.rs` also exercises
  `/api/docs/retrieval` through the builtin-registry external Modelica path,
  so the gateway-layer additive proof now covers mixed docs-facing retrieval
  as well as plain docs search
- that same `M6` additive slice now pushes deeper into the non-planner
  Studio docs route family too: `tests/unit/studio_repo_sync_api.rs` now
  also exercises `/api/docs/retrieval-context` through the builtin-registry
  external Modelica path, so the gateway-layer additive proof now covers
  deterministic node-context reopening as well
- that same `M6` additive slice now closes the sibling deterministic
  reopening peer too: `tests/unit/studio_repo_sync_api.rs` now also
  exercises `/api/docs/retrieval-hit` through the builtin-registry external
  Modelica path, so the gateway-layer additive proof now covers
  deterministic hit reopening as well
- that same `M6` additive slice now closes the deterministic page-lookup
  peer too: `tests/unit/studio_repo_sync_api.rs` now also exercises
  `/api/docs/page` through the builtin-registry external Modelica path, so
  the gateway-layer additive proof now covers deterministic docs page
  lookup as well
- that same `M6` additive slice now closes the deterministic family-context
  peer too: `tests/unit/studio_repo_sync_api.rs` now also exercises
  `/api/docs/family-context` through the builtin-registry external Modelica
  path, so the gateway-layer additive proof now covers grouped family
  context reopening as well
- that same `M6` additive slice now closes the deterministic family-search
  peer too: `tests/unit/studio_repo_sync_api.rs` now also exercises
  `/api/docs/family-search` through the builtin-registry external Modelica
  path, so the gateway-layer additive proof now covers grouped family
  search expansion as well
- that same `M6` additive slice now closes the deterministic family-cluster
  peer too: `tests/unit/studio_repo_sync_api.rs` now also exercises
  `/api/docs/family-cluster` through the builtin-registry external Modelica
  path, so the gateway-layer additive proof now covers single-family
  reopening as well
- that same `M6` additive slice now closes the deterministic navigation peer
  too: `tests/unit/studio_repo_sync_api.rs` now also exercises
  `/api/docs/navigation` through the builtin-registry external Modelica
  path, so the gateway-layer additive proof now covers tree-context plus
  family-cluster reopening as well
- that same `M6` additive slice now closes the deterministic navigation-
  search peer too: `tests/unit/studio_repo_sync_api.rs` now also exercises
  `/api/docs/navigation-search` through the builtin-registry external
  Modelica path, so the gateway-layer additive proof now covers grouped
  navigation-bundle expansion as well
- that same `M6` additive slice now closes the docs projected-gap-report
  peer too: `tests/unit/studio_repo_sync_api.rs` now also exercises
  `/api/docs/projected-gap-report` through the builtin-registry external
  Modelica path, so the gateway-layer additive proof now covers docs-facing
  gap reporting as well
- that same `M6` additive slice now opens the sibling Studio repo route
  family too: `tests/unit/studio_repo_sync_api.rs` now also exercises
  `/api/repo/overview` through the builtin-registry external Modelica path,
  so the gateway-layer additive proof now covers repo-summary reopening
  outside the docs-only surface too
- that same `M6` additive slice now closes the sibling Studio repo module-
  search peer too: `tests/unit/studio_repo_sync_api.rs` now also exercises
  `/api/repo/module-search` through the builtin-registry external Modelica
  path, so the gateway-layer additive proof now covers deterministic module
  search outside the docs-only surface too
- that same `M6` additive slice now closes the sibling Studio repo symbol-
  search peer too: `tests/unit/studio_repo_sync_api.rs` now also exercises
  `/api/repo/symbol-search` through the builtin-registry external Modelica
  path, so the gateway-layer additive proof now covers deterministic symbol
  search outside the docs-only surface too
- that same `M6` additive slice now closes the sibling Studio repo example-
  search peer too: `tests/unit/studio_repo_sync_api.rs` now also exercises
  `/api/repo/example-search` through the builtin-registry external Modelica
  path, so the gateway-layer additive proof now covers deterministic example
  search outside the docs-only surface too
- that same `M6` additive slice now closes the sibling Studio repo doc-
  coverage peer too: `tests/unit/studio_repo_sync_api.rs` now also exercises
  `/api/repo/doc-coverage` through the builtin-registry external Modelica
  path, so the gateway-layer additive proof now covers deterministic
  module-scoped doc coverage outside the docs-only surface too
- that same `M6` additive slice now closes a bundled Studio repo lifecycle-
  and-projection batch too: `tests/unit/studio_repo_sync_api.rs` now also
  exercises `/api/repo/sync`, `/api/repo/projected-pages`, and
  `/api/repo/projected-gap-report` through the builtin-registry external
  Modelica path, so the gateway-layer additive proof now covers repo
  status reopening, projected-page enumeration, and projected-gap reporting
  outside the docs-only surface too
- that same `M6` additive slice now closes a bundled deterministic Studio
  repo projected reopen family too: `tests/unit/studio_repo_sync_api.rs`
  now also exercises `/api/repo/projected-page`,
  `/api/repo/projected-page-index-tree`, `/api/repo/projected-page-index-node`,
  `/api/repo/projected-retrieval-hit`, and
  `/api/repo/projected-retrieval-context` through the builtin-registry
  external Modelica path, so the gateway-layer additive proof now covers
  symbol-page reopening, tree reopening, node reopening, deterministic hit
  reopening, and node-context reopening outside the docs-only surface too
- that same `M6` additive slice now closes the remaining bundled Studio repo
  projected query-and-navigation family too: `tests/unit/studio_repo_sync_api.rs`
  now also exercises `/api/repo/projected-page-index-tree-search`,
  `/api/repo/projected-page-search`, `/api/repo/projected-retrieval`,
  `/api/repo/projected-page-family-context`,
  `/api/repo/projected-page-family-search`,
  `/api/repo/projected-page-family-cluster`,
  `/api/repo/projected-page-navigation`,
  `/api/repo/projected-page-navigation-search`, and
  `/api/repo/projected-page-index-trees` through the builtin-registry
  external Modelica path, so the gateway-layer additive proof now also
  covers deterministic section search, projected page search, mixed
  projected retrieval, grouped family reopening, single-family reopening,
  navigation-bundle reopening, navigation-search expansion, and projected
  tree listing outside the docs-only surface too

## Current Compatibility Ledger

The current live tree now has a narrower set of Julia-named outward surfaces.
These should be treated as an explicit compatibility ledger rather than as
default host vocabulary.

### Legacy compatibility surfaces that still remain intentionally

No host-owned outward compatibility route, tool surface, or crate-root
compatibility namespace remains in the live artifact path. The remaining
Julia-specific compatibility imports are package-owned in
`packages/rust/crates/xiuxian-wendao-julia/src/compatibility/link_graph/`.

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
packages/rust/crates/xiuxian-wendao-julia/src/compatibility/
  link_graph/
```

The remaining compatibility import work is now package-owned under
`xiuxian-wendao-julia`.

The host crate-root compatibility shim is gone. The flat crate-root Julia
re-export block, the `runtime_config` compatibility sub-namespace, the former
`src/compatibility/julia.rs` shim, and the final
`src/compatibility/link_graph.rs` migration module are all retired from the
live tree. The remaining host-side legacy regression no longer imports any
crate-root compatibility helper path.

Downstream migration guidance now becomes:

1. prefer `xiuxian_wendao_julia::compatibility::link_graph::*` for
   Julia-specific deployment and launch compatibility DTO imports
2. treat host crate-root compatibility imports as retired from code
3. keep generic plugin-artifact inspection on the canonical `xiuxian-wendao`
   surfaces instead of restoring a host compatibility namespace

### Next removal / generalization candidates

The next outward surfaces most likely to move after `P1` are:

1. post-`M5` downstream import cleanup for any package-owned Julia
   compatibility DTO users
2. post-`M6` additive-proof expansion beyond the landed Modelica slice,
   reusing the canonical generic plugin-artifact and plugin-capability
   surfaces without reintroducing host-owned compatibility seams

## Stage-B Acceptance State

The outward-alignment bundle and exit review are complete when:

1. this inventory describes the live outward surfaces rather than only the
   early extraction map
2. the active docs agree that late-`M6` additive proof now spans repo-facing,
   docs-facing, and Studio-facing consumers plus the repo service-state bundle
3. `Phase 7` transport hardening is closed with a `go` decision
4. `Phase-8 Stage A` has verified the live `deny.toml` / `justfile` /
   Nix security baseline and the missing semver/dependency-hygiene lanes
5. `Phase-8 Stage B` has fixed the policy boundary: blocking workspace
   security, blocking `xiuxian-wendao-core` semver governance, and advisory
   Wendao-scoped dependency hygiene
6. `Phase-8 Stage C` has landed the first bounded lane bundle and the
   explicit `Phase 8` gate decision is now `go`

## Current Remediation Front

The active post-`Phase 8` work is no longer contract/governance setup. It is
bounded `xiuxian-daochang` crate-health remediation.

The latest three landed slices retired the first telegram/discord internal
visibility and re-export drift bundle, the stale `llm/*` import/owner
drift bundle, and the duplicate `runtime_agent_factory/*` owner bundle from
the compile front. The next live owner-drift front now starts in:

1. `xiuxian-daochang/src/agent/bootstrap/zhixing.rs`
2. `xiuxian-daochang/src/agent/turn_execution/react_loop/*`
3. `xiuxian-daochang/src/session/bounded_store/window_ops/*`
4. deeper `xiuxian-daochang/src/agent/*` lifecycle and tool/session drift
5. residual channel/runtime API drift under `xiuxian-daochang/src/channels/*`

That next bounded slice is now also closed for the `zhixing` reminder/bootstrap
surface: reminder queue backfill, queue-aware reminder polling, and reminder
notice rendering now live on `xiuxian-zhixing::ZhixingHeyi`, and the host
bootstrap no longer owns parallel reminder logic. The active next front is
now:

1. `xiuxian-daochang/src/agent/turn_execution/react_loop/*`
2. `xiuxian-daochang/src/session/bounded_store/window_ops/*`
3. deeper `xiuxian-daochang/src/agent/*` lifecycle and tool/session drift
4. residual channel/runtime API drift under `xiuxian-daochang/src/channels/*`

That next bounded slice is now closed for the
`agent/turn_execution/react_loop/*` surface too. The touched scope now uses
explicit crate-owned imports instead of wildcard or deep relative imports,
the live tool-dispatch call sites match current runtime signatures, and the
`turn_store` owner seam is mounted again so the react loop can see the live
append-turn path. The active next front is now:

1. `xiuxian-daochang/src/agent/persistence/turn_store/*`
2. `xiuxian-daochang/src/agent/session_context/window_ops/*`
3. `xiuxian-daochang/src/session/bounded_store/window_ops/*`
4. deeper `xiuxian-daochang/src/agent/*` lifecycle and tool/session drift
5. residual channel/runtime API drift under `xiuxian-daochang/src/channels/*`

:RELATIONS:
:LINKS: [[index]], [[06_roadmap/404_repo_intelligence_for_sciml_and_msl]], [[06_roadmap/405_large_rust_modularization]], [[docs/rfcs/2026-03-27-wendao-core-runtime-plugin-migration-rfc.md]], [[.data/blueprints/wendao_arrow_plugin_core_runtime_migration.md]]
:END:

---

:FOOTER:
:STANDARDS: v2.0
:LAST_SYNC: 2026-03-31
:END:
