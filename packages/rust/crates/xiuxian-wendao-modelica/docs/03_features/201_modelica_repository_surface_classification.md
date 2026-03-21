# Modelica Repository Surface Classification

:PROPERTIES:
:ID: 04a35aaf3303e72383bea88d06e4868a7412e333
:TYPE: FEATURE
:STATUS: ACTIVE
:END:

The Modelica plugin classifies repository paths before record projection so the default query surface stays focused on library/API entities instead of tutorials and support assets.

## Surface Classes

- `api`: library-facing modules and symbols that should participate in default `module.search` and `symbol.search`
- `example`: runnable examples, primarily under `Examples/`
- `documentation`: `UsersGuide` pages, `README*`, and inline `annotation(Documentation(...))` payloads
- `support`: support-only material such as `Examples/ExampleUtilities`, `Examples/.../Utilities`, and `Internal`

## Current Projection Rules

- `Examples/` models produce `ExampleRecord` values and `ExampleOf` relations
- `Examples/ExampleUtilities` and `Examples/.../Utilities` are treated as support-only and stay out of the default example and symbol surfaces
- support-only package modules also stay out of the default `module.search` surface
- `Internal` paths are treated as support-only and stay out of the default symbol and module surfaces
- `UsersGuide/` pages produce `DocRecord` values and `Documents` relations instead of default `SymbolRecord` values
- `package.order` influences module, example, and `UsersGuide` doc ordering but is excluded from `DocRecord` inventory

## UsersGuide Semantics

The plugin currently emits semantic `DocRecord.format` hints for common MSL guide topics:

- tutorial
- release notes
- reference and literature
- overview
- contact
- glossary
- concept
- parameter

Matching `_annotation` variants are emitted for inline documentation payloads extracted from `annotation(Documentation(...))`.

## Coverage Behavior

`UsersGuide` file docs and `UsersGuide` annotation docs link to:

- the owning functional module
- the visible `UsersGuide` module hierarchy

This allows `doc.coverage` to surface nested guide pages such as `Tutorial/*` without query-time heuristics.
