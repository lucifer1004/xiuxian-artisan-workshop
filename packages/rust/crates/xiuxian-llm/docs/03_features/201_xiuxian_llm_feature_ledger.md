# Feature Ledger

:PROPERTIES:
:ID: e6b7ee9ef15feb0247c80b15642e364409cf756c
:TYPE: FEATURE
:STATUS: DRAFT
:END:

Feature ledger for the `xiuxian-llm` library crate. Track user-facing or system-facing capabilities implemented in this package.

Promote concrete `xiuxian-llm` capabilities into this ledger as feature slices land.

## Feature Topology

- Default build profile: `provider-litellm`
- Optional local-runtime umbrella: `local-llm`
- Child layout: `local-llm = ["mistral.rs", "vision-dots"]`
- `local-llm` groups:
  - `mistral.rs` for `mistral` runtime helpers and the in-process `mistralrs` embedding SDK
  - `vision-dots` as the DeepSeek OCR subfeature inside the local runtime family
