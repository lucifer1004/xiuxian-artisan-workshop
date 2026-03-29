# Compatibility Retirement Ledger

:PROPERTIES:
:ID: wendao-compatibility-retirement-ledger
:PARENT: [[index]]
:TAGS: roadmap, migration, plugins, compatibility, ledger
:STATUS: ACTIVE
:END:

## Purpose

This note is the final missing program artifact for the Wendao
core/runtime/plugin migration program.

It defines what compatibility surfaces remain, why they remain, what phase
unlocks their retirement, and what the target retirement state is.

Primary references:

- `[[06_roadmap/412_core_runtime_plugin_program]]`
- `[[06_roadmap/409_core_runtime_plugin_surface_inventory]]`
- `[[06_roadmap/413_m2_core_extraction_package_list]]`
- `[[06_roadmap/414_m3_runtime_extraction_package_list]]`
- `[[06_roadmap/415_m4_julia_externalization_package_list]]`

## Retirement Rule

A compatibility surface may be retired only when:

1. its replacement ownership is physically landed
2. its replacement import or endpoint path is documented
3. the compatibility shim no longer carries primary implementation meaning
4. the relevant macro phase exit criteria are satisfied

## Retirement Ledger

| Compatibility surface | Current location | Retirement unlock | Target retirement state |
| :--- | :--- | :--- | :--- |
| Legacy Studio compatibility artifact JSON shape | `src/gateway/studio/router/handlers/capabilities/deployment.rs` | `M5` generic artifact/UI cutover | Remove the route-local compat JSON wrapper once generic plugin-artifact UI payloads are canonical on every remaining consumer path |

## Retirement Order

The expected retirement order is:

1. top-level crate re-export retirement
2. Studio/OpenAPI/Zhenfa outward compatibility retirement
3. final Julia helper compatibility retirement
4. final test-only compatibility seam retirement

This order keeps the widest public surfaces shrinking first and the narrow
regression seams shrinking last.

## Protected Compatibility Surfaces

The following compatibility seams should remain protected until their unlock
phase is complete:

1. `src/compatibility/`

No new primary implementation logic may be added there.

## Current M5 Status

The first `M5` outward-surface cutover is now physically landed:

1. Studio routing and OpenAPI inventory now expose the canonical generic
   plugin-artifact endpoint at
   `/api/ui/plugins/{plugin_id}/artifacts/{artifact_id}`
2. the compat deployment-artifact route remains live, but it now behaves as a
   wrapper over the generic plugin-artifact resolution/render path
3. Zhenfa now also exposes `wendao.plugin_artifact` as the canonical generic
   tool/RPC surface, while `wendao.compat_deployment_artifact` remains as the
   narrowed compat-specific export path over the same selector-based export
   path
4. the Studio UI compatibility DTO seam is now also generic-first:
   the compat route is built from `UiPluginArtifact`, and the remaining
   legacy JSON adaptation is now route-local rather than a dedicated type leaf
5. the canonical Studio schema-export seam now matches that same rule:
   `studio_type_collection()` exports the generic artifact types without
   promoting the compat artifact DTO into the primary TypeScript-facing
   collection
6. the remaining Studio compatibility DTO exposure is now also more explicit:
   the compat artifact DTO no longer rides through any compatibility
   namespace and instead stays behind the route-local adapter in
   `src/gateway/studio/router/handlers/capabilities/deployment.rs`
7. the remaining router-level Studio consumers have now narrowed too:
   legacy compat JSON-shape coverage stays in the compatibility leaf itself,
   while router-level tests assert the compat JSON payload generically instead
   of deserializing a Julia-named DTO directly
8. the compat handler seam has now narrowed further as well:
   the route layer no longer imports a Julia-named DTO directly and instead
   delegates adaptation through a route-local JSON wrapper over
   `UiPluginArtifact`
9. the Julia-named Studio compatibility Rust symbols are now retired too:
   the compat route still preserves the legacy JSON shape, but the internal
   adapter DTOs and builder are now compat-first rather than Julia-named
10. the dedicated Studio compatibility type leaf is now retired too:
    the remaining compat payload adaptation now lives directly in
    `src/gateway/studio/router/handlers/capabilities/deployment.rs`
11. that same route-local adapter is now thinner too:
    the compat route no longer keeps a parallel Rust DTO and instead filters
    and normalizes the serialized `UiPluginArtifact` payload at the JSON
    boundary
12. the test-only Studio Julia route/query shim is now retired too:
    `JuliaDeploymentArtifactQuery` and `get_julia_deployment_artifact` have
    been deleted, and legacy regression coverage now targets the compat
    handler directly
13. the OpenAPI Julia route-path aliases are now retired from code entirely:
    `API_UI_JULIA_DEPLOYMENT_ARTIFACT_*` have been deleted, and the route
    inventory now validates only the canonical plugin-artifact path plus the
    compat deployment-artifact path
14. the Zhenfa Julia outward tool name is now retired from code entirely:
    `wendao.julia_deployment_artifact` is gone from the live tool/RPC path,
    while the remaining outward surfaces are
    `wendao.plugin_artifact` and `wendao.compat_deployment_artifact`
15. the remaining Julia RPC/helper family in Zhenfa is now aligned to that
    same retirement track: the test-only RPC shim has been deleted, native
    deployment regression coverage hangs directly off `deployment.rs`, and the
    former Julia helper/type aliases no longer exist
16. the top-level crate and `runtime_config` Julia re-export blocks are now
    retired from code too: Julia-named DTOs and deployment helpers no longer
    leak through flat re-export blocks
17. the intermediate `src/link_graph/runtime_config/compatibility/`
    sub-namespace is now retired as well, so no public runtime-config helper
    path sits between the host tree and the crate-root compatibility namespace
18. the former crate-root Julia helper shim `src/compatibility/julia.rs` is
    now retired from code entirely: `src/compatibility/link_graph.rs` is the
    only remaining crate-root compatibility surface in the live tree
19. this means the retirement unlock for Studio/OpenAPI/Zhenfa compatibility
    is now in progress, but not yet complete, because the remaining outward
    consumers and legacy payload names still need to converge on the generic
    surface

## Completion Condition

The compatibility retirement program is complete when:

1. Julia-named host surfaces are compatibility-only or removed
2. generic plugin-artifact and plugin-capability surfaces are canonical
3. `xiuxian-wendao-julia` owns Julia-specific meaning physically
4. no migration blocker still depends on the monolithic crate boundary

:RELATIONS:
:LINKS: [[index]], [[06_roadmap/412_core_runtime_plugin_program]], [[06_roadmap/409_core_runtime_plugin_surface_inventory]], [[06_roadmap/413_m2_core_extraction_package_list]], [[06_roadmap/414_m3_runtime_extraction_package_list]], [[06_roadmap/415_m4_julia_externalization_package_list]]
:END:

---
