# Modelica Package Order Projection

:PROPERTIES:
:ID: 1e44503b74fe580968ecd2c5e21c09a04518129c
:TYPE: FEATURE
:STATUS: ACTIVE
:END:

`package.order` is treated as a first-class repository signal in the Modelica plugin rather than a filesystem detail.

## Why It Matters

Alphabetical ordering is often wrong for Modelica repositories. MSL and similar libraries use `package.order` to express canonical presentation and traversal order.

If the plugin drops that signal, downstream query surfaces become noisy:

- `module.search` loses authored package order
- `example.search` loses intended example progression
- `UsersGuide` pages appear in filesystem order instead of authored reading order

## Current Behavior

The plugin currently consumes discovered `package.order` files for:

- module ordering
- example ordering
- `UsersGuide` doc ordering

The Wendao common core preserves analyzer order for equal-score module and example matches so the ordering signal survives query-time tie breaking.

## Guardrails

- `package.order` affects ordering only
- `package.order` does not become a `DocRecord`
- missing `package.order` falls back to deterministic path-based ordering

## Next Tracking Questions

- whether ordering should also influence future projection-layer navigation trees
- whether additional Modelica support surfaces should preserve separate authored ordering instead of sharing the same fallback rules
