# External Modelica Plugin Boundary

:PROPERTIES:
:ID: fc770cd79042e381127345288282b269f69a31d7
:TYPE: CORE
:STATUS: ACTIVE
:END:

`xiuxian-wendao-modelica` is the first external Repo Intelligence plugin crate for `xiuxian-wendao`.

## Scope

The crate owns Modelica- and MSL-specific analysis that should not live in the Wendao common core:

- conservative discovery of Modelica packages and modules from `package.mo`
- lightweight declaration scanning for `.mo` files
- repository-surface classification for API, example, documentation, and support paths
- conservative extraction of `annotation(Documentation(...))` content
- Modelica-specific `Contains`, `Declares`, `ExampleOf`, and `Documents` relation construction

## Non-Goals

The crate does not own:

- git mirror or checkout lifecycle
- repository registration
- graph storage internals
- CLI or gateway contracts
- generic query result types

Those responsibilities remain in `xiuxian-wendao::repo_intelligence`.

## Integration Contract

The public surface stays intentionally small:

- `src/lib.rs` re-exports the plugin entry only
- `src/plugin/entry.rs` implements the Wendao plugin trait
- `src/plugin/analysis.rs` orchestrates repository analysis
- `src/plugin/discovery.rs` discovers records and package-order metadata
- `src/plugin/relations.rs` builds normalized relations
- `src/plugin/parsing.rs` keeps conservative Modelica parsing helpers and parser snapshots
- `src/plugin/types.rs` hosts internal plugin-only types

The plugin must project only normalized Wendao records and relations. It must not reach into private Wendao storage or gateway internals.

## Current Boundary Decision

Julia remains a native bridge inside `xiuxian-wendao`, while Modelica stays external. This preserves a stable extension boundary and keeps MSL-specific semantics outside the Wendao common core.
