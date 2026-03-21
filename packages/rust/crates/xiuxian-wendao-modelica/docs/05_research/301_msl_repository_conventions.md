# MSL Repository Conventions

:PROPERTIES:
:ID: 6e77b336d0d4785eec043224c75b7b1580b9a50e
:TYPE: RESEARCH
:STATUS: ACTIVE
:END:

The external Modelica plugin is built around a conservative reading of the Modelica Standard Library repository layout.

## Stable Signals Observed So Far

- `package.mo` defines package hierarchy and is reliable enough for module discovery
- `Examples/` usually signals runnable example models
- `UsersGuide/` usually signals guide-style documentation rather than default API symbols
- `package.order` carries canonical ordering that should be preserved for modules, examples, and guide pages
- inline `annotation(Documentation(...))` payloads are common enough to justify first-class extraction

## Design Implications

- repository layout can drive a useful first-pass graph without requiring full semantic Modelica compilation
- guide and support assets should be kept off the default API surface
- ordering information must be preserved by the plugin and by Wendao query tie-breaking
- doc coverage should be relation-driven, not inferred from filenames at query time

## Open Research Questions

- whether `BaseClasses` should remain on the default API surface or move to a separate support/internal classification
- how aggressively to infer topic hierarchy from `UsersGuide` subtree shapes beyond the currently recognized patterns
- how far the plugin should go on declaration parsing before a richer AST-backed Modelica layer becomes necessary
